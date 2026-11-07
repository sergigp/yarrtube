use crate::domain::category::Category;
use crate::domain::llm_server::OllamaUrl;
use crate::domain::preferences::CategorizationSettings;
use crate::domain::services::{
    VideoCategoryAssigner, VideoCategoryAssignerApi, VideoOwnerFinder, VideoOwnerFinderApi,
};
use crate::domain::video::{Video, VideoOwner};
use crate::domain::video_category::{
    CategorizationAnswer, CategorizationQuestion, VideoCategoryVerdict, pending_categories,
};
use crate::domain::video_summary::SummaryText;
use crate::infrastructure::repositories::ollama_llm_server_repository::LlmServerRepository;
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::VideoCategoryVerdictRepository;
use crate::infrastructure::repositories::sqlite_video_summary_repository::VideoSummaryRepository;
use crate::infrastructure::shared::system_clock::Clock;
use anyhow::Context;
use std::sync::Arc;
use tracing::info;

/// Why a video isn't categorized now. Nothing about it changes.
enum Skip {
    /// It shouldn't be, for now.
    NotWanted(&'static str),
    /// It should be, but reading what it needs failed.
    Failed(anyhow::Error),
}

impl From<anyhow::Error> for Skip {
    fn from(error: anyhow::Error) -> Self {
        Self::Failed(error)
    }
}

/// What every call of one categorization shares.
struct CategorizationJob {
    url: OllamaUrl,
    settings: CategorizationSettings,
    thinking: bool,
    owner: VideoOwner,
    summary: SummaryText,
    pending: Vec<Category>,
}

/// Asks the LLM server which categories a video fits — see the
/// `video-categorization` capability. Runs from the `categorize_video` task;
/// `VideoReconciler` decides when to schedule it.
#[derive(Clone)]
pub struct VideoCategorizer {
    preferences_repository: Arc<dyn PreferencesRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    category_repository: Arc<dyn CategoryRepository>,
    video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
    video_summary_repository: Arc<dyn VideoSummaryRepository>,
    llm_server_repository: Arc<dyn LlmServerRepository>,
    video_category_assigner: VideoCategoryAssigner,
    clock: Arc<dyn Clock>,
}

impl VideoCategorizer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        preferences_repository: Arc<dyn PreferencesRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        category_repository: Arc<dyn CategoryRepository>,
        video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
        video_summary_repository: Arc<dyn VideoSummaryRepository>,
        llm_server_repository: Arc<dyn LlmServerRepository>,
        video_category_assigner: VideoCategoryAssigner,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            preferences_repository,
            video_owner_finder,
            category_repository,
            video_category_verdict_repository,
            video_summary_repository,
            llm_server_repository,
            video_category_assigner,
            clock,
        }
    }
}

pub trait VideoCategorizerApi: Send + Sync {
    /// The categories pending for `video` now: with an AI prompt, not given
    /// by `owner`'s source, without a verdict for it.
    fn pending(&self, video: &Video, owner: &VideoOwner) -> anyhow::Result<Vec<Category>>;
    /// Asks the model about each category pending when it runs, storing
    /// each verdict as it comes. Err when any call was rejected, after
    /// trying all; on the last attempt those become unavailable. Holds no
    /// `VideoLock`: calls take minutes.
    fn categorize(&self, video: &Video, is_last_attempt: bool) -> anyhow::Result<()>;
}

impl VideoCategorizerApi for VideoCategorizer {
    fn pending(&self, video: &Video, owner: &VideoOwner) -> anyhow::Result<Vec<Category>> {
        let verdicts = self
            .video_category_verdict_repository
            .find_many(std::slice::from_ref(&video.id))?;
        Ok(pending_categories(
            self.category_repository.list()?,
            &owner.source,
            &verdicts,
        ))
    }

    fn categorize(&self, video: &Video, _is_last_attempt: bool) -> anyhow::Result<()> {
        let job = match self.prepare(video) {
            Ok(job) => job,
            Err(Skip::NotWanted(reason)) => {
                info!(video_id = %video.id, reason, "skipped video categorization");
                return Ok(());
            }
            Err(Skip::Failed(error)) => return Err(error),
        };
        job.pending
            .iter()
            .try_for_each(|category| self.evaluate(video, &job, category))
    }
}

impl VideoCategorizer {
    /// What the categorization needs, unless the video shouldn't be
    /// categorized now.
    fn prepare(&self, video: &Video) -> Result<CategorizationJob, Skip> {
        let preferences = self.preferences_repository.find()?.unwrap_or_default();
        if !preferences.categorization_available() {
            return Err(Skip::NotWanted("AI categorization is off"));
        }
        if !video.is_categorizable() {
            return Err(Skip::NotWanted("the video has no summary"));
        }
        let url = preferences.ollama_url.context("no Ollama URL is saved")?;
        let Some(owner) = self.video_owner_finder.find(video)? else {
            return Err(Skip::NotWanted("the video has no channel or playlist"));
        };
        if !owner.categorization_enabled {
            return Err(Skip::NotWanted(
                "its channel or playlist has categorization off",
            ));
        }
        let summary = self
            .video_summary_repository
            .find(&video.id)?
            .context("the video has no summary")?;
        let settings = preferences.categorization;
        let thinking = self
            .llm_server_repository
            .find(&url)?
            .model(settings.model.as_str())
            .with_context(|| format!("{} isn't installed on Ollama", settings.model.as_str()))?
            .thinking;
        let pending = self.pending(video, &owner)?;
        Ok(CategorizationJob {
            url,
            settings,
            thinking,
            owner,
            summary: summary.text,
            pending,
        })
    }

    /// Asks the model whether `video` fits `category` and applies its verdict.
    fn evaluate(
        &self,
        video: &Video,
        job: &CategorizationJob,
        category: &Category,
    ) -> anyhow::Result<()> {
        let question =
            CategorizationQuestion::new(video, &job.owner.source_name, &job.summary, category);
        let reply = self
            .llm_server_repository
            .chat(&job.url, &question.chat(&job.settings, job.thinking))?;
        let answer =
            CategorizationAnswer::parse(&reply.content).context("the answer isn't usable")?;
        let verdict = VideoCategoryVerdict::answered(
            video.id.clone(),
            category.id.clone(),
            answer,
            job.settings.model.as_str(),
            self.clock.now(),
        );
        self.video_category_assigner
            .apply_verdict(&verdict, &job.owner.source)?;
        Ok(())
    }
}
