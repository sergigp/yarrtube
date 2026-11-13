use crate::domain::category::Category;
use crate::domain::llm_server::{LlmConnection, LlmModel, LlmReply, LlmServerUnreachable};
use crate::domain::preferences::{AiProvider, Preferences};
use crate::domain::services::{
    CategorizationResetter, CategorizationResetterApi, VideoCategoryAssigner,
    VideoCategoryAssignerApi, VideoOwnerFinder, VideoOwnerFinderApi, VideoUpdater, VideoUpdaterApi,
};
use crate::domain::video::{
    StepStatus, UNAVAILABLE_SERVER_WAIT, Video, VideoOwner, VideoStepOutcome,
};
use crate::domain::video_category::{
    CategorizationAnswer, CategorizationCall, CategorizationQuestion, CategorizationTally, Verdict,
    VideoCategoryVerdict, pending_categories,
};
use crate::domain::video_summary::SummaryText;
use crate::infrastructure::repositories::llm_server_repository::LlmServerRepository;
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
    /// categorized as it stands (Done, or Failed when a category was given
    /// up), without checking the server.
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

/// How a call thinks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Thinking {
    /// As the job asks (its settings and the model).
    AsAsked,
    /// Off: a call with thinking about these categories stopped at the
    /// output limit.
    Dropped,
}

/// A category without an answer from one call, with why.
struct Rejection<'a> {
    category: &'a Category,
    reason: String,
    kind: RejectionKind,
}

impl Rejection<'_> {
    /// How the category is asked again alone: without thinking once a call
    /// about it dropped thinking.
    fn thinking(&self) -> Thinking {
        match self.kind {
            RejectionKind::WithoutThinking => Thinking::Dropped,
            RejectionKind::Retryable | RejectionKind::ThinkingRanOut => Thinking::AsAsked,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RejectionKind {
    /// Refused, past its timeout, unusable, or at the limit without having
    /// thought: alone in the fallback, then a later attempt.
    Retryable,
    /// Its call thought and stopped at the output limit: asked again
    /// without thinking.
    ThinkingRanOut,
    /// Rejected by a call made with `Thinking::Dropped`: alone without
    /// thinking if the call was about several, else given up now.
    WithoutThinking,
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
    connection: LlmConnection,
    call: CategorizationCall,
    /// Always `false` on Anthropic: "Retry Without Thinking" never applies.
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
    video_updater: VideoUpdater,
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
    /// each verdict as it comes. A call that thinks and stops at the output
    /// limit is asked once more without thinking; a category that call
    /// rejects is given up at once (a `failed` verdict). A run with any
    /// other category rejected is a failed attempt under the `video-retries`
    /// policy. Ollama that can't be connected to, or hasn't the model,
    /// re-queues it later without touching the video or writing `failed`
    /// verdicts.
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
                self.finish(video)
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
        let connection = preferences.connection().context("no Ollama URL is saved")?;
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
        let pending = self.pending(video, &owner)?;
        if pending.is_empty() {
            return Err(Skip::NothingPending);
        }
        let job = CategorizationJob {
            call: self.unchecked_call(&preferences)?,
            preferences,
            connection,
            thinking: false,
            owner,
            summary: summary.text,
            pending,
        };
        let model = self.check_server(&job)?;
        Ok(Self::checked(job, &model))
    }

    /// The call as saved, before the server says how the model is: what a
    /// failed check records its failed attempt with.
    fn unchecked_call(&self, preferences: &Preferences) -> anyhow::Result<CategorizationCall> {
        let settings = &preferences.categorization;
        Ok(match preferences.ai_provider {
            AiProvider::Ollama => CategorizationCall::ollama(
                settings,
                settings.effective_context_length(self.prompted_categories()?),
            ),
            AiProvider::Anthropic => CategorizationCall::anthropic(
                &preferences.anthropic,
                settings.effective_prompt(),
                &LlmModel {
                    name: preferences
                        .anthropic
                        .categorization_model
                        .as_str()
                        .to_string(),
                    display_name: None,
                    size_bytes: None,
                    thinking: false,
                    context_window: None,
                },
            ),
        })
    }

    /// `job` as the checked `model` calls it: with thinking on Ollama when
    /// its settings want it and the model supports it.
    fn checked(job: CategorizationJob, model: &LlmModel) -> CategorizationJob {
        match job.preferences.ai_provider {
            AiProvider::Ollama => CategorizationJob {
                thinking: job.preferences.categorization.thinking && model.thinking,
                ..job
            },
            AiProvider::Anthropic => CategorizationJob {
                call: CategorizationCall::anthropic(
                    &job.preferences.anthropic,
                    job.call.prompt.clone(),
                    model,
                ),
                thinking: false,
                ..job
            },
        }
    }

    /// Checks the server before any call, whatever the settings: the saved
    /// model as the server has it. Waits when the provider is unavailable
    /// or hasn't the model; a failed attempt when the check otherwise fails.
    fn check_server(&self, job: &CategorizationJob) -> Result<LlmModel, Skip> {
        let model = job.call.model.as_str();
        self.llm_server_repository
            .find(&job.connection)
            .map_err(|error| Skip::from_server_error(error, job))?
            .model(model)
            .cloned()
            .with_context(|| job.connection.missing_model(model))
            .map_err(Skip::Unavailable)
    }

    /// Re-queues the categorization once Ollama may be back, leaving the
    /// video and its verdicts as they are: no attempt spent.
    fn wait_for_server(&self, video: &Video, reason: anyhow::Error) -> VideoStepOutcome {
        warn!(video_id = %video.id, %reason, "the AI provider is unavailable, waiting to categorize video");
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

    /// Asks about every pending category; one rejected without thinking is
    /// given up at once, any other rejected, or a server fault stopping it,
    /// is a failed attempt. Ollama becoming unreachable stops it to wait,
    /// keeping the verdicts already stored.
    fn ask_each(&self, video: &Video, job: &CategorizationJob) -> anyhow::Result<VideoStepOutcome> {
        let rejected = match self.rejected_categories(video, job) {
            Ok(rejected) => rejected,
            Err(Stop::Unavailable(reason)) => return Ok(self.wait_for_server(video, reason)),
            Err(Stop::Server(reason)) => return self.fail_everything(video, job, reason),
            Err(Stop::Failed(error)) => return Err(error),
        };
        self.log_tally(video, job, rejected.len())?;
        let (given_up, retryable): (Vec<_>, Vec<_>) = rejected
            .into_iter()
            .partition(|rejection| rejection.kind == RejectionKind::WithoutThinking);
        self.give_up(video, job, &given_up)?;
        match Self::fail_if_rejected(&retryable) {
            Ok(()) => self.settle(video, job, !given_up.is_empty()),
            Err(cause) => {
                self.fail_attempt(video, job, &retryable, error_report::cause_chain(&cause))
            }
        }
    }

    /// Asks about each pending category in turn, once more without thinking
    /// after a call thought past the limit; the ones rejected, with why.
    fn rejected_categories<'a>(
        &self,
        video: &Video,
        job: &'a CategorizationJob,
    ) -> Result<Vec<Rejection<'a>>, Stop> {
        let rejected = self.ask(video, job, &job.pending, Call::First, Thinking::AsAsked)?;
        let rejected =
            self.retry_without_thinking(video, job, &job.pending, rejected, Call::First)?;
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
        thinking: Thinking,
    ) -> Result<Vec<Rejection<'a>>, Stop> {
        let started = Instant::now();
        let dropped = thinking == Thinking::Dropped;
        let thinks = job.thinking && !dropped;
        let question =
            CategorizationQuestion::new(video, &job.owner.source_name, &job.summary, categories);
        let reply = self
            .llm_server_repository
            .chat(&job.connection, &question.chat(&job.call, thinks))
            .map_err(Stop::from_chat_error)?;
        Self::log_call(video, categories, call, thinking, thinks, &reply, started);
        if reply.stopped_at_limit {
            Self::log_limit_stop(video, categories, thinks, &reply);
        }
        let rejected_kind = if dropped {
            RejectionKind::WithoutThinking
        } else {
            RejectionKind::Retryable
        };
        let rejection = Self::call_rejection(&reply, &question, job, thinks, rejected_kind);
        if let Some((reason, kind)) = rejection {
            return Ok(Self::rejected_all(categories, &reason, kind));
        }
        categories
            .iter()
            .zip(question.answers(&reply.content))
            .filter_map(|(category, (_, answer))| match answer {
                Some(answer) => self
                    .store(video, job, category, answer)
                    .err()
                    .map(|error| Err(Stop::Failed(error))),
                None => Some(Ok(Rejection {
                    category,
                    reason: "its answer isn't usable".to_string(),
                    kind: rejected_kind,
                })),
            })
            .collect()
    }

    /// `rejected` of the call about every pending category, each asked
    /// again alone when that call was about several; the ones rejected again.
    fn fall_back<'a>(
        &self,
        video: &Video,
        job: &CategorizationJob,
        rejected: Vec<Rejection<'a>>,
    ) -> Result<Vec<Rejection<'a>>, Stop> {
        let rejected = match job.pending.len() {
            1 => rejected,
            _ => rejected
                .into_iter()
                .map(|rejection| {
                    Self::log_falling_back(video, rejection.category, &rejection.reason);
                    let alone = std::slice::from_ref(rejection.category);
                    let rejected =
                        self.ask(video, job, alone, Call::Fallback, rejection.thinking())?;
                    self.retry_without_thinking(video, job, alone, rejected, Call::Fallback)
                })
                .collect::<Result<Vec<_>, Stop>>()?
                .into_iter()
                .flatten()
                .collect(),
        };
        rejected.iter().for_each(|rejection| match rejection.kind {
            RejectionKind::WithoutThinking => Self::log_given_up(video, rejection),
            RejectionKind::Retryable | RejectionKind::ThinkingRanOut => {
                Self::log_rejected(video, rejection.category, &rejection.reason)
            }
        });
        Ok(rejected)
    }

    /// `categories` asked once more, in one call without thinking, when
    /// the call about them thought and stopped at the output limit
    /// (`rejected` is then every one of them); else `rejected` unchanged.
    fn retry_without_thinking<'a>(
        &self,
        video: &Video,
        job: &CategorizationJob,
        categories: &'a [Category],
        rejected: Vec<Rejection<'a>>,
        call: Call,
    ) -> Result<Vec<Rejection<'a>>, Stop> {
        if rejected
            .iter()
            .any(|rejection| rejection.kind == RejectionKind::ThinkingRanOut)
        {
            return self.ask(video, job, categories, call, Thinking::Dropped);
        }
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
            &job.call.model,
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

    /// Why the whole call is rejected, and how, if it is: refused, or
    /// stopped at the output limit (`ThinkingRanOut` when it thought).
    fn call_rejection(
        reply: &LlmReply,
        question: &CategorizationQuestion,
        job: &CategorizationJob,
        thinks: bool,
        rejected_kind: RejectionKind,
    ) -> Option<(String, RejectionKind)> {
        let limit_kind = if thinks {
            RejectionKind::ThinkingRanOut
        } else {
            rejected_kind
        };
        match (&reply.refusal, reply.stopped_at_limit) {
            (Some(refusal), _) => Some((
                format!("the LLM provider refused the call: {refusal}"),
                rejected_kind,
            )),
            (None, true) => Some((
                format!(
                    "it hit the {}-token output limit",
                    question.max_output_tokens(job.call.context_length)
                ),
                limit_kind,
            )),
            (None, false) => None,
        }
    }

    /// Every one of `categories`, rejected for `reason`.
    fn rejected_all<'a>(
        categories: &'a [Category],
        reason: &str,
        kind: RejectionKind,
    ) -> Vec<Rejection<'a>> {
        categories
            .iter()
            .map(|category| Rejection {
                category,
                reason: reason.to_string(),
                kind,
            })
            .collect()
    }

    fn log_call(
        video: &Video,
        categories: &[Category],
        call: Call,
        thinking: Thinking,
        thinks: bool,
        reply: &LlmReply,
        started: Instant,
    ) {
        info!(
            video_id = %video.id,
            categories = %Self::names(categories),
            fallback = matches!(call, Call::Fallback),
            thinking = thinks,
            retry_without_thinking = thinking == Thinking::Dropped,
            input_tokens = reply.input_tokens,
            output_tokens = reply.output_tokens,
            stopped_at_limit = reply.stopped_at_limit,
            duration_seconds = started.elapsed().as_secs_f64(),
            "asked the model"
        );
    }

    /// One line counting what the run asked and got, from the verdicts it
    /// stored.
    fn log_tally(
        &self,
        video: &Video,
        job: &CategorizationJob,
        rejected: usize,
    ) -> anyhow::Result<()> {
        let verdicts = self
            .video_category_verdict_repository
            .find_many(std::slice::from_ref(&video.id))?;
        let tally = CategorizationTally::new(&job.pending, &verdicts, rejected);
        info!(
            video_id = %video.id,
            asked = tally.asked,
            yes = tally.yes,
            no = tally.no,
            rejected = tally.rejected,
            "categorized video"
        );
        Ok(())
    }

    /// `categories`' names, comma-separated.
    fn names(categories: &[Category]) -> String {
        categories
            .iter()
            .map(|category| category.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// How a call that stopped at the output limit spent its output.
    fn log_limit_stop(video: &Video, categories: &[Category], thinking: bool, reply: &LlmReply) {
        warn!(
            video_id = %video.id,
            categories = %Self::names(categories),
            thinking,
            thinking_chars = reply.thinking.chars().count(),
            answer_chars = reply.content.chars().count(),
            thinking_tail = %reply.thinking_tail(),
            "the model stopped at the output limit"
        );
    }

    fn log_given_up(video: &Video, rejection: &Rejection) {
        error!(
            video_id = %video.id,
            category = %rejection.category.name.as_str(),
            reason = %format!(
                "it thought past the output limit, then without thinking {}",
                rejection.reason
            ),
            "gave up on a category"
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
        let rejected = Self::rejected_all(&job.pending, &reason, RejectionKind::Retryable);
        self.fail_attempt(video, job, &rejected, reason)
    }

    /// Records a failed attempt, failed for `reason`: Retrying, run again
    /// when its next attempt is due, or Failed, giving each of `rejected`
    /// a `failed` verdict (see `give_up`).
    fn fail_attempt(
        &self,
        video: &Video,
        job: &CategorizationJob,
        rejected: &[Rejection],
        reason: String,
    ) -> anyhow::Result<VideoStepOutcome> {
        let recorded = self.record_attempt_failure(video)?;
        let recorded = if recorded
            .as_ref()
            .is_some_and(|video| video.categorization.status == StepStatus::Failed)
        {
            self.give_up(video, job, rejected)?;
            self.reset_if_still_pending(video, job, recorded)?
        } else {
            recorded
        };
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
        video: &Video,
        job: &CategorizationJob,
        recorded: Option<Video>,
    ) -> anyhow::Result<Option<Video>> {
        if self.pending(video, &job.owner)?.is_empty() {
            return Ok(recorded);
        }
        self.categorization_resetter
            .reset(std::slice::from_ref(&video.id))?;
        Ok(None)
    }

    /// After a run that left nothing to retry: Done, or Failed when the
    /// video holds a `failed` verdict (Pending if one given up was dropped).
    fn settle(
        &self,
        video: &Video,
        job: &CategorizationJob,
        gave_up: bool,
    ) -> anyhow::Result<VideoStepOutcome> {
        if gave_up && !self.pending(video, &job.owner)?.is_empty() {
            self.categorization_resetter
                .reset(std::slice::from_ref(&video.id))?;
            return Ok(VideoStepOutcome::Settled);
        }
        self.finish(video)
    }

    /// Records a run that left nothing to ask: Failed when the video holds a
    /// `failed` verdict, else Done.
    fn finish(&self, video: &Video) -> anyhow::Result<VideoStepOutcome> {
        if self.holds_failed_verdict(video)? {
            self.mark_given_up(video)
        } else {
            self.mark_categorized(video)
        }
    }

    /// Records a run that left only categories given up on: Failed.
    fn mark_given_up(&self, video: &Video) -> anyhow::Result<VideoStepOutcome> {
        let now = self.clock.now();
        self.video_updater
            .update(&video.id, |video| Some(video.give_up_categorization(now)))?;
        Ok(VideoStepOutcome::Settled)
    }

    /// Whether the AI gave up on any of `video`'s pairs.
    fn holds_failed_verdict(&self, video: &Video) -> anyhow::Result<bool> {
        Ok(self
            .video_category_verdict_repository
            .find_many(std::slice::from_ref(&video.id))?
            .iter()
            .any(|verdict| verdict.verdict == Verdict::Failed))
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
        rejected: &[Rejection],
    ) -> anyhow::Result<()> {
        let now = self.clock.now();
        rejected.iter().try_for_each(|Rejection { category, .. }| {
            let verdict = VideoCategoryVerdict::failed(
                video.id.clone(),
                category.id.clone(),
                &job.call.model,
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
    fn fail_if_rejected(rejected: &[Rejection]) -> anyhow::Result<()> {
        if rejected.is_empty() {
            return Ok(());
        }
        let causes = rejected
            .iter()
            .map(|rejection| {
                format!(
                    "{} ({})",
                    rejection.category.name.as_str(),
                    rejection.reason
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        Err(anyhow!("the model couldn't answer for {causes}"))
    }
}
