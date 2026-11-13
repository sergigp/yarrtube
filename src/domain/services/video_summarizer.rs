use crate::domain::event::DomainEvent;
use crate::domain::llm_server::{
    Effort, LlmChat, LlmConnection, LlmModel, LlmServerUnreachable, MIN_CHAT_TIMEOUT,
};
use crate::domain::preferences::{
    AiProvider, AnthropicSettings, ContextLength, Preferences, SummaryModel, SummaryPrompt,
    SummarySettings,
};
use crate::domain::services::{
    VideoOwnerFinder, VideoOwnerFinderApi, VideoUpdater, VideoUpdaterApi,
};
use crate::domain::video::{
    UNAVAILABLE_SERVER_WAIT, Video, VideoOwner, VideoStepOutcome, VideoSummaryGenerated,
};
use crate::domain::video_summary::{
    PartAnswer, SummaryInput, SummaryMessages, SummaryText, VideoSummary,
};
use crate::domain::video_transcript::{CaptionLanguage, TimedTranscript};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::llm_server_repository::LlmServerRepository;
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::sqlite_video_summary_repository::VideoSummaryRepository;
use crate::infrastructure::repositories::sqlite_video_transcript_repository::VideoTranscriptRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::error_report;
use crate::infrastructure::shared::system_clock::Clock;
use anyhow::{Context, anyhow};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};

/// Cap on the Anthropic context the input budget is computed from.
const ANTHROPIC_MAX_CONTEXT: u32 = 131_072;
const ANTHROPIC_SUMMARY_MAX_OUTPUT_TOKENS: u32 = 16_000;

/// What every call of one generation shares.
struct SummaryJob {
    connection: LlmConnection,
    call: SummaryCall,
    prompt: SummaryPrompt,
    owner: VideoOwner,
}

/// What each call of one generation sends, resolved from the provider.
struct SummaryCall {
    model: SummaryModel,
    context_length: ContextLength,
    max_output_tokens: u32,
    /// Ollama only.
    temperature: f64,
    /// Ollama only.
    thinking: bool,
    /// Anthropic only.
    effort: Effort,
}

impl SummaryCall {
    /// The saved Ollama settings, thinking when the `installed` model can:
    /// the answer may use all the context the prompt leaves.
    fn ollama(settings: &SummarySettings, installed: &LlmModel) -> Self {
        Self {
            model: settings.model.clone(),
            context_length: settings.context_length,
            max_output_tokens: settings.context_length.value(),
            temperature: settings.temperature.value(),
            thinking: settings.thinking && installed.thinking,
            effort: Effort::default(),
        }
    }

    /// The saved Anthropic settings, for the model as Anthropic `listed` it:
    /// the input budget comes from its context window, capped.
    fn anthropic(settings: &AnthropicSettings, listed: &LlmModel) -> Self {
        let window = listed
            .context_window
            .map_or(ANTHROPIC_MAX_CONTEXT, |window| {
                window.min(ANTHROPIC_MAX_CONTEXT)
            });
        Self {
            model: settings.summaries_model.clone(),
            context_length: ContextLength::new(window.into()).unwrap_or_default(),
            max_output_tokens: ANTHROPIC_SUMMARY_MAX_OUTPUT_TOKENS,
            temperature: 0.0,
            thinking: false,
            effort: settings.summaries_effort,
        }
    }
}

/// Why a video isn't summarized now. Nothing about the video changes.
enum Skip {
    /// It shouldn't be, for now or for good.
    NotWanted(&'static str),
    /// It should be, but something outside the video stops it.
    Blocked(anyhow::Error),
    /// Ollama can't be connected to or hasn't the saved model: wait for it,
    /// no attempt spent.
    Unavailable(anyhow::Error),
    /// The server check connected but failed: a failed attempt.
    Failed(anyhow::Error),
}

/// A call that stopped at the context limit on more input than the budget.
struct Overrun {
    call: String,
    /// None when Ollama refused the call: it reports no count then.
    input_tokens: Option<u32>,
    /// The call's messages' characters over its input tokens.
    chars_per_token: f64,
    /// The usual "ran out of context" error, if it isn't cut again.
    reason: anyhow::Error,
}

/// Why a generation failed.
enum Failure {
    /// Ollama can't be connected to during a call: wait for it, no attempt
    /// spent.
    Unavailable(anyhow::Error),
    /// The server's fault: a later sync can try again.
    Server(anyhow::Error),
    /// This video didn't get a summary out of the settings as they are.
    Rejected(anyhow::Error),
    /// The input ran over the budget: cut it smaller, once.
    OverBudget(Overrun),
}

impl Failure {
    /// What a failed chat call means: Ollama unavailable, or its fault.
    fn from_chat_error(error: anyhow::Error) -> Self {
        if error.is::<LlmServerUnreachable>() {
            Self::Unavailable(error)
        } else {
            Self::Server(error)
        }
    }
}

impl From<anyhow::Error> for Failure {
    fn from(error: anyhow::Error) -> Self {
        Self::Rejected(error)
    }
}

impl Skip {
    /// What a failed server check means: Ollama unavailable, or a failed
    /// attempt.
    fn from_server_error(error: anyhow::Error) -> Self {
        if error.is::<LlmServerUnreachable>() {
            Self::Unavailable(error)
        } else {
            Self::Failed(error)
        }
    }
}

impl From<anyhow::Error> for Skip {
    fn from(error: anyhow::Error) -> Self {
        Self::Blocked(error)
    }
}

/// Writes a video's AI summary through the LLM server — see the
/// `video-summaries` capability. Runs from the `summarize_video` task;
/// `VideoReconciler` decides when to schedule it.
#[derive(Clone)]
pub struct VideoSummarizer {
    video_repository: Arc<dyn VideoRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    preferences_repository: Arc<dyn PreferencesRepository>,
    video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    llm_server_repository: Arc<dyn LlmServerRepository>,
    video_summary_repository: Arc<dyn VideoSummaryRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
    video_updater: VideoUpdater,
}

impl VideoSummarizer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        preferences_repository: Arc<dyn PreferencesRepository>,
        video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        llm_server_repository: Arc<dyn LlmServerRepository>,
        video_summary_repository: Arc<dyn VideoSummaryRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_updater: VideoUpdater::new(video_repository.clone()),
            video_repository,
            video_owner_finder,
            preferences_repository,
            video_transcript_repository,
            video_file_repository,
            llm_server_repository,
            video_summary_repository,
            event_publisher,
            clock,
        }
    }
}

pub trait VideoSummarizerApi: Send + Sync {
    /// See the `video-summaries` capability. A rejected call, a server fault
    /// or an unreadable transcript is a failed attempt, recorded on the video
    /// and re-queued under the `video-retries` policy. Ollama that can't be
    /// connected to, or hasn't the model, re-queues it later without touching
    /// the video.
    fn summarize(&self, video: &Video) -> anyhow::Result<VideoStepOutcome>;
}

impl VideoSummarizerApi for VideoSummarizer {
    fn summarize(&self, video: &Video) -> anyhow::Result<VideoStepOutcome> {
        match self.prepare(video) {
            Ok(job) => self.generate(video, &job),
            Err(Skip::NotWanted(reason)) => {
                info!(video_id = %video.id, reason, "skipped video summary");
                Ok(VideoStepOutcome::Settled)
            }
            Err(Skip::Blocked(reason)) => {
                warn!(video_id = %video.id, %reason, "couldn't summarize video");
                Ok(VideoStepOutcome::Settled)
            }
            Err(Skip::Unavailable(reason)) => Ok(self.wait_for_server(video, reason)),
            Err(Skip::Failed(reason)) => {
                warn!(video_id = %video.id, %reason, "couldn't summarize video");
                self.record_failure(video, error_report::cause_chain(&reason))
            }
        }
    }
}

impl VideoSummarizer {
    /// What the generation needs, unless the video shouldn't or can't be
    /// summarized now.
    fn prepare(&self, video: &Video) -> Result<SummaryJob, Skip> {
        let preferences = self.preferences_repository.find()?.unwrap_or_default();
        if !preferences.summaries_available() {
            return Err(Skip::NotWanted("AI summaries are off"));
        }
        if !video.is_summarizable() {
            return Err(Skip::NotWanted("the video doesn't need a summary"));
        }
        let connection = preferences.connection().context("no Ollama URL is saved")?;
        let Some(owner) = self.video_owner_finder.find(video)? else {
            return Err(Skip::NotWanted("the video has no channel or playlist"));
        };
        if !owner.summaries_enabled {
            return Err(Skip::NotWanted("its channel or playlist has summaries off"));
        }
        let call = self.summary_call(&connection, &preferences)?;
        let prompt = preferences
            .summaries
            .effective_prompt()
            .validated()
            .map_err(anyhow::Error::new)?;
        Ok(SummaryJob {
            connection,
            call,
            prompt,
            owner,
        })
    }

    /// How the saved provider's settings call the model the server has.
    fn summary_call(
        &self,
        connection: &LlmConnection,
        preferences: &Preferences,
    ) -> Result<SummaryCall, Skip> {
        match preferences.ai_provider {
            AiProvider::Ollama => {
                let settings = &preferences.summaries;
                let installed = self.check_server(connection, &settings.model)?;
                Ok(SummaryCall::ollama(settings, &installed))
            }
            AiProvider::Anthropic => {
                let settings = &preferences.anthropic;
                let listed = self.check_server(connection, &settings.summaries_model)?;
                Ok(SummaryCall::anthropic(settings, &listed))
            }
        }
    }

    /// The saved model on the provider behind `connection`, unless it is
    /// unavailable or hasn't it (wait) or the check fails (a failed attempt).
    fn check_server(
        &self,
        connection: &LlmConnection,
        model: &SummaryModel,
    ) -> Result<LlmModel, Skip> {
        let server = self
            .llm_server_repository
            .find(connection)
            .map_err(Skip::from_server_error)?;
        let model = model.as_str();
        server
            .model(model)
            .cloned()
            .with_context(|| connection.missing_model(model))
            .map_err(Skip::Unavailable)
    }

    /// Re-queues the summary once Ollama may be back, leaving the video as
    /// it is: no attempt spent.
    fn wait_for_server(&self, video: &Video, reason: anyhow::Error) -> VideoStepOutcome {
        warn!(video_id = %video.id, %reason, "the AI provider is unavailable, waiting to summarize video");
        VideoStepOutcome::RetryAt {
            at: self.clock.now() + UNAVAILABLE_SERVER_WAIT,
            reason: error_report::cause_chain(&reason),
        }
    }

    fn generate(&self, video: &Video, job: &SummaryJob) -> anyhow::Result<VideoStepOutcome> {
        match self.try_generate(video, job) {
            Ok(()) => Ok(VideoStepOutcome::Settled),
            Err(Failure::Unavailable(reason)) => Ok(self.wait_for_server(video, reason)),
            Err(
                Failure::Server(cause)
                | Failure::Rejected(cause)
                | Failure::OverBudget(Overrun { reason: cause, .. }),
            ) => {
                error!(video_id = %video.id, error = format!("{cause:#}"), "failed to summarize video");
                self.record_failure(video, error_report::cause_chain(&cause))
            }
        }
    }

    /// Records a failed attempt, failed for `reason`, on the video as
    /// stored (unless it no longer wants a summary): Retrying, run again
    /// when its next attempt is due, or Failed.
    fn record_failure(&self, video: &Video, reason: String) -> anyhow::Result<VideoStepOutcome> {
        let now = self.clock.now();
        let recorded = self.video_updater.update(&video.id, |video| {
            video
                .is_summarizable()
                .then(|| video.record_summary_failure(now))
        })?;
        Ok(VideoStepOutcome::retry_at_or_settled(
            recorded.and_then(|video| video.summary.next_attempt_at()),
            reason,
        ))
    }

    fn try_generate(&self, video: &Video, job: &SummaryJob) -> Result<(), Failure> {
        let started = Instant::now();
        let (language, transcript) = self.transcript(video, job)?;
        let text = self.write_summary_cut_smaller_once(video, job, &language, &transcript)?;
        if self.still_wanted(video)?.is_none() {
            info!(video_id = %video.id, "video no longer wants a summary, dropping the generated one");
            return Ok(());
        }
        let now = self.clock.now();
        self.video_summary_repository.save(&VideoSummary::create(
            video.id.clone(),
            text.clone(),
            &job.call.model,
            now,
        ))?;
        let marked = self.video_updater.update(&video.id, |video| {
            video
                .is_summarizable()
                .then(|| video.mark_summary_generated(now))
        })?;
        if marked.is_some() {
            self.publish_generated(video);
        }
        info!(
            video_id = %video.id,
            summary = %text.as_str(),
            duration_seconds = started.elapsed().as_secs_f64(),
            video_seconds = video.download.duration_seconds(),
            model = %job.call.model.as_str(),
            "video summary generated"
        );
        Ok(())
    }

    /// Best effort, like `VideoSubtitlesFetched`: a lost event only delays
    /// the categorization to the next reconcile pass.
    fn publish_generated(&self, video: &Video) {
        let event = DomainEvent::VideoSummaryGenerated(VideoSummaryGenerated {
            video_id: video.id.as_str().to_string(),
        });
        if let Err(e) = self.event_publisher.publish(&event) {
            warn!(video_id = %video.id, error = %e, "failed to publish that the video's summary was generated");
        }
    }

    /// The stored video when the generated summary should still be stored
    /// now the minutes-long generation is done: the video still exists and
    /// is summarizable (not deleted, reset for redownload or already
    /// summarized), AI summaries are still on, and its source still has them
    /// enabled. Settings or the source's toggle may have changed while it
    /// generated — mirrors `VideoCategorizer`'s re-check before storing.
    fn still_wanted(&self, video: &Video) -> anyhow::Result<Option<Video>> {
        let Some(stored) = self
            .video_repository
            .find(&video.id)?
            .filter(Video::is_summarizable)
        else {
            return Ok(None);
        };
        if !self
            .preferences_repository
            .find()?
            .unwrap_or_default()
            .summaries_available()
        {
            return Ok(None);
        }
        let enabled = self
            .video_owner_finder
            .find(&stored)?
            .is_some_and(|owner| owner.summaries_enabled);
        Ok(enabled.then_some(stored))
    }

    /// One call for a whole transcript; for one in parts, a call per part
    /// and one over their answers. Stops at the first failed call.
    fn write_summary(
        &self,
        video: &Video,
        input: &SummaryInput,
        job: &SummaryJob,
    ) -> Result<SummaryText, Failure> {
        let parts = input.parts();
        if parts == 1 {
            return self.ask(video, job, input, "whole", input.whole());
        }
        let answers = (0..parts)
            .map(|index| {
                let call = format!("part {} of {parts}", index + 1);
                self.ask(video, job, input, &call, input.part(index))
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.reduced(video, input, job, input.answers(answers))
    }

    /// The final answer: what doesn't fit one call is first reduced in
    /// groups, round after round.
    fn reduced(
        &self,
        video: &Video,
        input: &SummaryInput,
        job: &SummaryJob,
        answers: Vec<PartAnswer>,
    ) -> Result<SummaryText, Failure> {
        if answers.len() == 1 || input.reduce_fits(&answers) {
            return self.ask(video, job, input, "reduce", input.reduce(&answers));
        }
        let groups = input.reduce_groups(&answers);
        let total = groups.len();
        let shrunk = groups
            .iter()
            .enumerate()
            .map(|(index, group)| match group.as_slice() {
                [only] => Ok(only.clone()),
                _ => {
                    let call = format!("reduce group {} of {total}", index + 1);
                    let messages = input.reduce_group(group);
                    self.ask(video, job, input, &call, messages)
                        .map(|summary| PartAnswer::merged(group, summary))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.reduced(video, input, job, shrunk)
    }

    fn ask(
        &self,
        video: &Video,
        job: &SummaryJob,
        input: &SummaryInput,
        call: &str,
        messages: SummaryMessages,
    ) -> Result<SummaryText, Failure> {
        info!(
            video_id = %video.id,
            title = %video.title,
            call,
            "sending prompt to the model"
        );
        let started = Instant::now();
        let context_length = job.call.context_length;
        let message_chars = messages.chars();
        let reply = self
            .llm_server_repository
            .chat(
                &job.connection,
                &LlmChat {
                    model: job.call.model.as_str().to_string(),
                    system: messages.system,
                    user: messages.user,
                    context_length: context_length.value(),
                    temperature: job.call.temperature,
                    thinking: job.call.thinking,
                    effort: job.call.effort,
                    max_output_tokens: job.call.max_output_tokens,
                    format: None,
                    timeout: MIN_CHAT_TIMEOUT,
                },
            )
            .map_err(Failure::from_chat_error)?;
        info!(
            video_id = %video.id,
            call,
            answer = %reply.content,
            thinking = job.call.thinking,
            input_tokens = reply.input_tokens,
            output_tokens = reply.output_tokens,
            stopped_at_limit = reply.stopped_at_limit,
            duration_seconds = started.elapsed().as_secs_f64(),
            "model answered"
        );
        if let Some(refusal) = reply.refusal {
            return Err(match reply.exceeded_context {
                true => Self::exceeded_context(call, &context_length, message_chars, &refusal),
                false => anyhow!("Ollama refused the call: {refusal}").into(),
            });
        }
        if reply.stopped_at_limit {
            warn!(
                video_id = %video.id,
                call,
                thinking = job.call.thinking,
                thinking_chars = reply.thinking.chars().count(),
                answer_chars = reply.content.chars().count(),
                thinking_tail = %reply.thinking_tail(),
                "the model stopped at the output limit"
            );
            return Err(Self::out_of_context(
                call,
                input,
                &context_length,
                message_chars,
                reply.input_tokens,
            ));
        }
        SummaryText::from_answer(&reply.content).map_err(|e| anyhow::Error::new(e).into())
    }

    /// `write_summary`; on an overrun, once more cut at its measured ratio.
    fn write_summary_cut_smaller_once(
        &self,
        video: &Video,
        job: &SummaryJob,
        language: &CaptionLanguage,
        transcript: &TimedTranscript,
    ) -> Result<SummaryText, Failure> {
        let input =
            self.summary_input(video, job, language, transcript, language.chars_per_token());
        Self::log_generating(video, job, &input);
        match self.write_summary(video, &input, job) {
            Err(Failure::OverBudget(overrun)) => {
                Self::log_cut_smaller(video, &input, &overrun);
                let smaller =
                    self.summary_input(video, job, language, transcript, overrun.chars_per_token);
                Self::log_generating(video, job, &smaller);
                self.write_summary(video, &smaller, job)
            }
            written => written,
        }
    }

    fn log_cut_smaller(video: &Video, input: &SummaryInput, overrun: &Overrun) {
        info!(
            video_id = %video.id,
            call = %overrun.call,
            input_tokens = %overrun
                .input_tokens
                .map_or_else(|| "over the context length".to_string(), |tokens| tokens.to_string()),
            budget_tokens = input.budget_tokens(),
            chars_per_token = overrun.chars_per_token,
            "input over the budget, cutting the transcript smaller"
        );
    }

    /// A call that stopped at the context limit: over the budget, it can be
    /// cut smaller; within it, the model ran out of room.
    fn out_of_context(
        call: &str,
        input: &SummaryInput,
        context_length: &ContextLength,
        message_chars: usize,
        input_tokens: u32,
    ) -> Failure {
        let reason = anyhow!(
            "the model ran out of context ({} tokens: {} of input, {} left to think and answer)",
            context_length.value(),
            input_tokens,
            context_length.value().saturating_sub(input_tokens)
        );
        match input.over_budget(input_tokens) {
            true => Failure::OverBudget(Overrun {
                call: call.to_string(),
                input_tokens: Some(input_tokens),
                chars_per_token: message_chars as f64 / f64::from(input_tokens),
                reason,
            }),
            false => Failure::Rejected(reason),
        }
    }

    /// A call Ollama refused because the prompt exceeds the context length:
    /// the real ratio is under its characters over the context length.
    fn exceeded_context(
        call: &str,
        context_length: &ContextLength,
        message_chars: usize,
        refusal: &str,
    ) -> Failure {
        Failure::OverBudget(Overrun {
            call: call.to_string(),
            input_tokens: None,
            chars_per_token: message_chars as f64 / f64::from(context_length.value()),
            reason: anyhow!("Ollama refused the call: {refusal}"),
        })
    }

    fn summary_input(
        &self,
        video: &Video,
        job: &SummaryJob,
        language: &CaptionLanguage,
        transcript: &TimedTranscript,
        chars_per_token: f64,
    ) -> SummaryInput {
        SummaryInput::new(
            video,
            language,
            transcript,
            &job.call.context_length,
            &job.prompt,
            chars_per_token,
        )
    }

    fn log_generating(video: &Video, job: &SummaryJob, input: &SummaryInput) {
        info!(
            video_id = %video.id,
            title = %video.title,
            video_seconds = video.download.duration_seconds(),
            parts = input.parts(),
            model = %job.call.model.as_str(),
            "generating video summary"
        );
    }

    /// The video's transcript language and its timed lines, from the
    /// subtitle file.
    fn transcript(
        &self,
        video: &Video,
        job: &SummaryJob,
    ) -> anyhow::Result<(CaptionLanguage, TimedTranscript)> {
        let transcript = self
            .video_transcript_repository
            .find(&video.id)?
            .context("video has no stored transcript")?;
        let filename = video
            .subtitles
            .filename
            .as_deref()
            .context("video has no subtitle file")?;
        let srt = self
            .video_file_repository
            .read_to_string(&job.owner.target.output_dir, filename)?
            .context("video's subtitle file is gone")?;
        let timed = TimedTranscript::from_srt(&srt);
        anyhow::ensure!(!timed.is_empty(), "video's subtitle file has no text");
        Ok((transcript.language, timed))
    }
}
