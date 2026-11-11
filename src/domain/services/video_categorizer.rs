use crate::domain::category::Category;
use crate::domain::llm_server::{LlmReply, LlmServerUnreachable, OllamaUrl};
use crate::domain::preferences::{CategorizationSettings, ContextLength, Preferences};
use crate::domain::services::{
    CategorizationResetter, VideoCategoryAssigner, VideoCategoryAssignerApi, VideoOwnerFinder,
    VideoOwnerFinderApi, VideoUpdater, VideoUpdaterApi,
};
use crate::domain::video::{
    StepStatus, UNAVAILABLE_SERVER_WAIT, Video, VideoOwner, VideoStepOutcome,
};
use crate::domain::video_category::{
    CategorizationAnswer, CategorizationQuestion, VideoCategoryVerdict, pending_categories,
};
use crate::domain::video_summary::SummaryText;
use crate::infrastructure::repositories::ollama_llm_server_repository::LlmServerRepository;
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::VideoCategoryVerdictRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::sqlite_video_summary_repository::VideoSummaryRepository;
use crate::infrastructure::shared::error_report;
use crate::infrastructure::shared::system_clock::Clock;
use anyhow::{Context, anyhow};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};

/// Why the model isn't asked about a video now. Nothing about the video
/// changes, except that with nothing pending it is marked categorized.
enum Skip {
    /// It shouldn't be, for now.
    NotWanted(&'static str),
    /// No category is pending for the video: nothing to ask, so it is
    /// categorized as it stands (Done), without checking the server.
    NothingPending,
    /// It should be, but something outside the video stops it.
    Blocked(anyhow::Error),
    /// Ollama can't be connected to or hasn't the saved model: wait for it,
    /// no attempt spent.
    Unavailable(anyhow::Error),
    /// The server check connected but failed: a failed attempt of the job
    /// that would have run.
    Failed(anyhow::Error, Box<CategorizationJob>),
}

impl Skip {
    /// What a failed server check means: Ollama unavailable, or a failed
    /// attempt of `job`.
    fn from_server_error(error: anyhow::Error, job: &CategorizationJob) -> Self {
        if error.is::<LlmServerUnreachable>() {
            Self::Unavailable(error)
        } else {
            Self::Failed(error, Box::new(job.clone()))
        }
    }
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
    /// Ollama can't be connected to during a call: wait for it, no attempt
    /// spent.
    Unavailable(anyhow::Error),
    /// The server's fault: a later sync tries again.
    Server(anyhow::Error),
    /// Storing failed.
    Failed(anyhow::Error),
}

impl Stop {
    /// What a failed chat call means: Ollama unavailable, or its fault.
    fn from_chat_error(error: anyhow::Error) -> Self {
        if error.is::<LlmServerUnreachable>() {
            Self::Unavailable(error)
        } else {
            Self::Server(error)
        }
    }
}

impl From<anyhow::Error> for Stop {
    fn from(error: anyhow::Error) -> Self {
        Self::Failed(error)
    }
}

/// What every call of one categorization shares.
#[derive(Clone)]
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
    video_updater: VideoUpdater,
    #[allow(dead_code)]
    categorization_resetter: CategorizationResetter,
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
        video_repository: Arc<dyn VideoRepository>,
        categorization_resetter: CategorizationResetter,
    ) -> Self {
        Self {
            video_updater: VideoUpdater::new(video_repository),
            categorization_resetter,
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
    /// failed attempt under the `video-retries` policy. Ollama that can't be
    /// connected to, or hasn't the model, re-queues it later without touching
    /// the video or writing `failed` verdicts.
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
            Err(Skip::NothingPending) => {
                info!(
                    video_id = %video.id,
                    reason = "no category is pending",
                    "skipped video categorization"
                );
                self.mark_categorized(video)
            }
            Err(Skip::Blocked(reason)) => {
                warn!(video_id = %video.id, %reason, "couldn't categorize video");
                Ok(VideoStepOutcome::Settled)
            }
            Err(Skip::Unavailable(reason)) => Ok(self.wait_for_server(video, reason)),
            Err(Skip::Failed(reason, job)) => self.fail_everything(video, &job, reason),
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
        if video.summary.status != StepStatus::Done {
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
        let pending = self.pending(video, &owner)?;
        if pending.is_empty() {
            return Err(Skip::NothingPending);
        }
        let context_length = settings.effective_context_length(self.prompted_categories()?);
        let job = CategorizationJob {
            preferences,
            url,
            settings,
            thinking: false,
            context_length,
            owner,
            summary: summary.text,
            pending,
        };
        let thinking = self.check_server(&job)?;
        Ok(CategorizationJob { thinking, ..job })
    }

    /// Checks the server before any call, whatever the settings: whether
    /// `job` asks with thinking (its settings want it and the saved model
    /// supports it). Waits when Ollama can't be connected to or hasn't the
    /// model; a failed attempt when the check otherwise fails.
    fn check_server(&self, job: &CategorizationJob) -> Result<bool, Skip> {
        let model = job.settings.model.as_str();
        self.llm_server_repository
            .find(&job.url)
            .map_err(|error| Skip::from_server_error(error, job))?
            .model(model)
            .map(|installed| job.settings.thinking && installed.thinking)
            .with_context(|| format!("{model} isn't installed on Ollama"))
            .map_err(Skip::Unavailable)
    }

    /// Re-queues the categorization once Ollama may be back, leaving the
    /// video and its verdicts as they are: no attempt spent.
    fn wait_for_server(&self, video: &Video, reason: anyhow::Error) -> VideoStepOutcome {
        warn!(video_id = %video.id, %reason, "Ollama is unavailable, waiting to categorize video");
        VideoStepOutcome::RetryAt {
            at: self.clock.now() + UNAVAILABLE_SERVER_WAIT,
            reason: error_report::cause_chain(&reason),
        }
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

    /// Asks about every pending category; any rejected, or a server fault
    /// stopping it, is a failed attempt. Ollama becoming unreachable stops
    /// it to wait, keeping the verdicts already stored.
    fn ask_each(&self, video: &Video, job: &CategorizationJob) -> anyhow::Result<VideoStepOutcome> {
        let rejected = match self.rejected_categories(video, job) {
            Ok(rejected) => rejected,
            Err(Stop::Unavailable(reason)) => return Ok(self.wait_for_server(video, reason)),
            Err(Stop::Server(reason)) => return self.fail_everything(video, job, reason),
            Err(Stop::Failed(error)) => return Err(error),
        };
        match Self::fail_if_rejected(&rejected) {
            Ok(()) => self.mark_categorized(video),
            Err(cause) => {
                self.fail_attempt(video, job, &rejected, error_report::cause_chain(&cause))
            }
        }
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
            .map_err(Stop::from_chat_error)?;
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
            1 => rejected,
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

    /// Records a failed attempt with every pending category rejected, for
    /// `reason`: the server check or a call failed with a server fault.
    fn fail_everything(
        &self,
        video: &Video,
        job: &CategorizationJob,
        reason: anyhow::Error,
    ) -> anyhow::Result<VideoStepOutcome> {
        warn!(video_id = %video.id, %reason, "couldn't categorize video");
        let reason = error_report::cause_chain(&reason);
        let rejected = Self::rejected_all(&job.pending, &reason);
        self.fail_attempt(video, job, &rejected, reason)
    }

    /// Records a failed attempt, failed for `reason`: Retrying, run again
    /// when its next attempt is due, or Failed, giving each of `rejected`
    /// a `failed` verdict (see `give_up`).
    fn fail_attempt(
        &self,
        video: &Video,
        job: &CategorizationJob,
        rejected: &[(&Category, String)],
        reason: String,
    ) -> anyhow::Result<VideoStepOutcome> {
        let recorded = self.record_attempt_failure(video)?;
        if recorded
            .as_ref()
            .is_some_and(|video| video.categorization.status == StepStatus::Failed)
        {
            self.give_up(video, job, rejected)?;
        }
        let recorded = self.reset_if_still_pending(video, job, recorded)?;
        Ok(VideoStepOutcome::retry_at_or_settled(
            recorded.and_then(|video| video.categorization.next_attempt_at()),
            reason,
        ))
    }

    /// After a give-up: back to Pending when a category is still pending
    /// (its `failed` verdict dropped because the settings or its prompt
    /// changed during the attempt); the video as stored after.
    fn reset_if_still_pending(
        &self,
        _video: &Video,
        _job: &CategorizationJob,
        recorded: Option<Video>,
    ) -> anyhow::Result<Option<Video>> {
        Ok(recorded)
    }

    /// Records a run that left nothing rejected: Done.
    fn mark_categorized(&self, video: &Video) -> anyhow::Result<VideoStepOutcome> {
        let now = self.clock.now();
        self.video_updater
            .update(&video.id, |video| Some(video.mark_categorized(now)))?;
        Ok(VideoStepOutcome::Settled)
    }

    /// The video as stored once one more failed attempt is recorded on it.
    fn record_attempt_failure(&self, video: &Video) -> anyhow::Result<Option<Video>> {
        let now = self.clock.now();
        self.video_updater.update(&video.id, |video| {
            Some(video.record_categorization_failure(now))
        })
    }

    /// Gives each of `rejected` a `failed` verdict once the categorization
    /// gave up, unless it is no longer pending as the job started (see
    /// `still_pending`).
    fn give_up(
        &self,
        video: &Video,
        job: &CategorizationJob,
        rejected: &[(&Category, String)],
    ) -> anyhow::Result<()> {
        let now = self.clock.now();
        rejected.iter().try_for_each(|(category, _)| {
            let verdict = VideoCategoryVerdict::failed(
                video.id.clone(),
                category.id.clone(),
                job.settings.model.as_str(),
                now,
            );
            self.video_category_assigner
                .apply_verdict_if(&verdict, &job.owner.source, &|| {
                    self.still_pending(video, job, category)
                })
                .map(|_| ())
        })
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
