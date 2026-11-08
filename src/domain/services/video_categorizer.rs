use crate::domain::category::Category;
use crate::domain::llm_server::OllamaUrl;
use crate::domain::preferences::{CategorizationSettings, Preferences};
use crate::domain::services::{
    VideoCategoryAssigner, VideoCategoryAssignerApi, VideoOwnerFinder, VideoOwnerFinderApi,
};
use crate::domain::video::{Video, VideoOwner};
use crate::domain::video_category::categorization_question::MAX_OUTPUT_TOKENS;
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
use anyhow::{Context, anyhow};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};

/// Why a video isn't categorized now. Nothing about it changes.
enum Skip {
    /// It shouldn't be, for now.
    NotWanted(&'static str),
    /// It should be, but something outside the video stops it.
    Blocked(anyhow::Error),
}

impl From<anyhow::Error> for Skip {
    fn from(error: anyhow::Error) -> Self {
        Self::Blocked(error)
    }
}

/// How one call ended, when it didn't stop the categorization.
enum Outcome {
    /// Its verdict was applied.
    Answered,
    /// The model didn't answer usably, for this reason: retrying may.
    Rejected(String),
}

/// Why the categorization stops before trying every category.
enum Stop {
    /// The server's fault: a later sync tries again.
    Server(anyhow::Error),
    /// Storing failed.
    Failed(anyhow::Error),
}

impl From<anyhow::Error> for Stop {
    fn from(error: anyhow::Error) -> Self {
        Self::Failed(error)
    }
}

/// What every call of one categorization shares.
struct CategorizationJob {
    preferences: Preferences,
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

    fn categorize(&self, video: &Video, is_last_attempt: bool) -> anyhow::Result<()> {
        match self.prepare(video) {
            Ok(job) => self.ask_each(video, &job, is_last_attempt),
            Err(Skip::NotWanted(reason)) => {
                info!(video_id = %video.id, reason, "skipped video categorization");
                Ok(())
            }
            Err(Skip::Blocked(reason)) => {
                warn!(video_id = %video.id, %reason, "couldn't categorize video");
                Ok(())
            }
        }
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
        let url = preferences
            .ollama_url
            .clone()
            .context("no Ollama URL is saved")?;
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
        let settings = preferences.categorization.clone();
        let thinking = settings.thinking
            && self
                .llm_server_repository
                .find(&url)?
                .model(settings.model.as_str())
                .with_context(|| format!("{} isn't installed on Ollama", settings.model.as_str()))?
                .thinking;
        let pending = self.pending(video, &owner)?;
        Ok(CategorizationJob {
            preferences,
            url,
            settings,
            thinking,
            owner,
            summary: summary.text,
            pending,
        })
    }

    /// Asks about every pending category, then fails naming the rejected
    /// ones, after giving them up on the last attempt. A server fault stops
    /// it quietly.
    fn ask_each(
        &self,
        video: &Video,
        job: &CategorizationJob,
        is_last_attempt: bool,
    ) -> anyhow::Result<()> {
        let rejected = match self.rejected_categories(video, job) {
            Ok(rejected) => rejected,
            Err(Stop::Server(reason)) => {
                warn!(video_id = %video.id, %reason, "couldn't categorize video");
                return Ok(());
            }
            Err(Stop::Failed(error)) => return Err(error),
        };
        if is_last_attempt {
            self.give_up(video, job, &rejected)?;
        }
        Self::fail_if_rejected(&rejected)
    }

    /// Asks about each pending category in turn; the ones rejected, with why.
    fn rejected_categories<'a>(
        &self,
        video: &Video,
        job: &'a CategorizationJob,
    ) -> Result<Vec<(&'a Category, String)>, Stop> {
        job.pending
            .iter()
            .filter_map(|category| match self.evaluate(video, job, category) {
                Ok(Outcome::Answered) => None,
                Ok(Outcome::Rejected(reason)) => {
                    error!(
                        video_id = %video.id,
                        category = %category.name.as_str(),
                        %reason,
                        "the model couldn't answer for a category"
                    );
                    Some(Ok((category, reason)))
                }
                Err(stop) => Some(Err(stop)),
            })
            .collect()
    }

    /// Asks the model whether `video` fits `category` and applies its verdict.
    fn evaluate(
        &self,
        video: &Video,
        job: &CategorizationJob,
        category: &Category,
    ) -> Result<Outcome, Stop> {
        let started = Instant::now();
        let question =
            CategorizationQuestion::new(video, &job.owner.source_name, &job.summary, category);
        let reply = self
            .llm_server_repository
            .chat(&job.url, &question.chat(&job.settings, job.thinking))
            .map_err(Stop::Server)?;
        if let Some(refusal) = reply.refusal {
            return Ok(Outcome::Rejected(format!(
                "Ollama refused the call: {refusal}"
            )));
        }
        if reply.stopped_at_limit {
            return Ok(Outcome::Rejected(format!(
                "it hit the {MAX_OUTPUT_TOKENS}-token output limit"
            )));
        }
        let Some(answer) = CategorizationAnswer::parse(&reply.content) else {
            return Ok(Outcome::Rejected("its answer isn't usable".to_string()));
        };
        let verdict = VideoCategoryVerdict::answered(
            video.id.clone(),
            category.id.clone(),
            answer,
            job.settings.model.as_str(),
            self.clock.now(),
        );
        let stored =
            self.video_category_assigner
                .apply_verdict_if(&verdict, &job.owner.source, &|| {
                    self.still_pending(video, job, category)
                })?;
        if !stored {
            info!(
                video_id = %video.id,
                category = %category.name.as_str(),
                "dropped a verdict: the pair was decided or changed during its call"
            );
            return Ok(Outcome::Answered);
        }
        info!(
            video_id = %video.id,
            category = %category.name.as_str(),
            verdict = %verdict.verdict.as_str(),
            reason = %verdict.reason.as_deref().unwrap_or_default(),
            input_tokens = reply.input_tokens,
            duration_seconds = started.elapsed().as_secs_f64(),
            "video categorized"
        );
        Ok(Outcome::Answered)
    }

    /// Gives each of `rejected` an unavailable verdict, unless it is no
    /// longer pending as the job started (see `still_pending`).
    fn give_up(
        &self,
        video: &Video,
        job: &CategorizationJob,
        rejected: &[(&Category, String)],
    ) -> anyhow::Result<()> {
        rejected.iter().try_for_each(|(category, _)| {
            let verdict = VideoCategoryVerdict::unavailable(
                video.id.clone(),
                category.id.clone(),
                job.settings.model.as_str(),
                self.clock.now(),
            );
            self.video_category_assigner
                .apply_verdict_if(&verdict, &job.owner.source, &|| {
                    self.still_pending(video, job, category)
                })
                .map(|_| ())
        })
    }

    /// Whether `video`'s pair with `category` is still as the job found it:
    /// without a verdict (the user may have decided it), with the same
    /// settings and prompt (a change, then a re-run, wants the new ones).
    fn still_pending(
        &self,
        video: &Video,
        job: &CategorizationJob,
        category: &Category,
    ) -> anyhow::Result<bool> {
        let decided = self
            .video_category_verdict_repository
            .find_many(std::slice::from_ref(&video.id))?
            .iter()
            .any(|verdict| verdict.category_id == category.id);
        Ok(!decided && self.unchanged_since(job, category)?)
    }

    /// Whether the categorization settings and `category`'s prompt are
    /// still those `job` started with.
    fn unchanged_since(
        &self,
        job: &CategorizationJob,
        category: &Category,
    ) -> anyhow::Result<bool> {
        let preferences = self.preferences_repository.find()?.unwrap_or_default();
        let current = self.category_repository.find(&category.id)?;
        Ok(!job.preferences.changes_categorization(&preferences)
            && current.is_some_and(|current| current.ai_prompt == category.ai_prompt))
    }

    /// An error naming each of `rejected`, with why; `Ok` when none was.
    fn fail_if_rejected(rejected: &[(&Category, String)]) -> anyhow::Result<()> {
        if rejected.is_empty() {
            return Ok(());
        }
        let causes = rejected
            .iter()
            .map(|(category, reason)| format!("{} ({reason})", category.name.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        Err(anyhow!("the model couldn't answer for {causes}"))
    }
}
