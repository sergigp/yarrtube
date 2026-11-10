use crate::domain::category::Category;
use crate::domain::llm_server::{LlmReply, OllamaUrl};
use crate::domain::preferences::{CategorizationSettings, ContextLength, Preferences};
use crate::domain::services::{
    VideoCategoryAssigner, VideoCategoryAssignerApi, VideoOwnerFinder, VideoOwnerFinderApi,
};
use crate::domain::video::{Video, VideoOwner, VideoStepOutcome};
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

/// Which call of a categorization: the one about every pending category,
/// or one about a category it left without an answer.
#[derive(Clone, Copy)]
enum Call {
    First,
    Fallback,
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
    context_length: ContextLength,
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
    /// each verdict as it comes. A run with any category rejected is a
    /// failed attempt under the `video-retries` policy.
    fn categorize(&self, video: &Video) -> anyhow::Result<VideoStepOutcome>;
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

    fn categorize(&self, video: &Video) -> anyhow::Result<VideoStepOutcome> {
        match self.prepare(video) {
            Ok(job) => self.ask_each(video, &job),
            Err(Skip::NotWanted(reason)) => {
                info!(video_id = %video.id, reason, "skipped video categorization");
                Ok(VideoStepOutcome::Settled)
            }
            Err(Skip::Blocked(reason)) => {
                warn!(video_id = %video.id, %reason, "couldn't categorize video");
                Ok(VideoStepOutcome::Settled)
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
        let context_length = settings.effective_context_length(self.prompted_categories()?);
        Ok(CategorizationJob {
            preferences,
            url,
            settings,
            thinking,
            context_length,
            owner,
            summary: summary.text,
            pending,
        })
    }

    /// How many categories have an AI prompt: what Auto sizes the context
    /// length from.
    fn prompted_categories(&self) -> anyhow::Result<usize> {
        Ok(self
            .category_repository
            .list()?
            .iter()
            .filter(|category| category.ai_prompt.is_some())
            .count())
    }

    /// Asks about every pending category, then fails naming the rejected
    /// ones. A server fault stops it quietly.
    fn ask_each(&self, video: &Video, job: &CategorizationJob) -> anyhow::Result<VideoStepOutcome> {
        let rejected = match self.rejected_categories(video, job) {
            Ok(rejected) => rejected,
            Err(Stop::Server(reason)) => {
                warn!(video_id = %video.id, %reason, "couldn't categorize video");
                return Ok(VideoStepOutcome::Settled);
            }
            Err(Stop::Failed(error)) => return Err(error),
        };
        Self::fail_if_rejected(&rejected).map(|_| VideoStepOutcome::Settled)
    }

    /// Asks about each pending category in turn; the ones rejected, with why.
    fn rejected_categories<'a>(
        &self,
        video: &Video,
        job: &'a CategorizationJob,
    ) -> Result<Vec<(&'a Category, String)>, Stop> {
        let rejected = self.ask(video, job, &job.pending, Call::First)?;
        self.fall_back(video, job, rejected)
    }

    /// One call about `categories`, storing each usable verdict; the ones
    /// rejected, with why.
    fn ask<'a>(
        &self,
        video: &Video,
        job: &CategorizationJob,
        categories: &'a [Category],
        call: Call,
    ) -> Result<Vec<(&'a Category, String)>, Stop> {
        let started = Instant::now();
        let question =
            CategorizationQuestion::new(video, &job.owner.source_name, &job.summary, categories);
        let reply = self
            .llm_server_repository
            .chat(
                &job.url,
                &question.chat(&job.settings, job.context_length, job.thinking),
            )
            .map_err(Stop::Server)?;
        Self::log_call(video, categories, call, &reply, started);
        let rejection = match (&reply.refusal, reply.stopped_at_limit) {
            (Some(refusal), _) => Some(format!("Ollama refused the call: {refusal}")),
            (None, true) => Some(format!(
                "it hit the {}-token output limit",
                question.max_output_tokens(job.context_length)
            )),
            (None, false) => None,
        };
        if let Some(reason) = rejection {
            return Ok(Self::rejected_all(categories, &reason));
        }
        categories
            .iter()
            .zip(question.answers(&reply.content))
            .filter_map(|(category, (_, answer))| match answer {
                Some(answer) => self
                    .store(video, job, category, answer)
                    .err()
                    .map(|error| Err(Stop::Failed(error))),
                None => Some(Ok((category, "its answer isn't usable".to_string()))),
            })
            .collect()
    }

    /// `rejected` of the call about every pending category, each asked
    /// again alone when that call was about several; the ones rejected again.
    fn fall_back<'a>(
        &self,
        video: &Video,
        job: &CategorizationJob,
        rejected: Vec<(&'a Category, String)>,
    ) -> Result<Vec<(&'a Category, String)>, Stop> {
        let rejected = match job.pending.len() {
            0 | 1 => rejected,
            _ => rejected
                .into_iter()
                .map(|(category, reason)| {
                    Self::log_falling_back(video, category, &reason);
                    self.ask(video, job, std::slice::from_ref(category), Call::Fallback)
                })
                .collect::<Result<Vec<_>, Stop>>()?
                .into_iter()
                .flatten()
                .collect(),
        };
        rejected
            .iter()
            .for_each(|(category, reason)| Self::log_rejected(video, category, reason));
        Ok(rejected)
    }

    /// Applies `category`'s verdict, unless the pair changed during the call.
    fn store(
        &self,
        video: &Video,
        job: &CategorizationJob,
        category: &Category,
        answer: CategorizationAnswer,
    ) -> anyhow::Result<()> {
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
            return Ok(());
        }
        info!(
            video_id = %video.id,
            category = %category.name.as_str(),
            verdict = %verdict.verdict.as_str(),
            reason = %verdict.reason.as_deref().unwrap_or_default(),
            "video categorized"
        );
        Ok(())
    }

    /// Every one of `categories`, rejected for `reason`.
    fn rejected_all<'a>(categories: &'a [Category], reason: &str) -> Vec<(&'a Category, String)> {
        categories
            .iter()
            .map(|category| (category, reason.to_string()))
            .collect()
    }

    fn log_call(
        video: &Video,
        categories: &[Category],
        call: Call,
        reply: &LlmReply,
        started: Instant,
    ) {
        let names = categories
            .iter()
            .map(|category| category.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        info!(
            video_id = %video.id,
            categories = %names,
            fallback = matches!(call, Call::Fallback),
            input_tokens = reply.input_tokens,
            output_tokens = reply.output_tokens,
            duration_seconds = started.elapsed().as_secs_f64(),
            "asked the model"
        );
    }

    fn log_falling_back(video: &Video, category: &Category, reason: &str) {
        warn!(
            video_id = %video.id,
            category = %category.name.as_str(),
            %reason,
            "the model couldn't answer for a category, asking it alone"
        );
    }

    fn log_rejected(video: &Video, category: &Category, reason: &str) {
        error!(
            video_id = %video.id,
            category = %category.name.as_str(),
            %reason,
            "the model couldn't answer for a category"
        );
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
