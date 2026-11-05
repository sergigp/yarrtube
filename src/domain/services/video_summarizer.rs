use crate::domain::llm_server::{LlmChat, OllamaUrl};
use crate::domain::preferences::{SummaryPrompt, SummarySettings};
use crate::domain::services::{VideoOwnerFinder, VideoOwnerFinderApi};
use crate::domain::video::{Video, VideoOwner};
use crate::domain::video_summary::{
    PartAnswer, SummaryInput, SummaryMessages, SummaryText, VideoSummary,
};
use crate::domain::video_transcript::TimedTranscript;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::ollama_llm_server_repository::LlmServerRepository;
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::sqlite_video_summary_repository::VideoSummaryRepository;
use crate::infrastructure::repositories::sqlite_video_transcript_repository::VideoTranscriptRepository;
use crate::infrastructure::shared::system_clock::Clock;
use anyhow::{Context, anyhow};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};

/// What every call of one generation shares.
struct SummaryJob {
    url: OllamaUrl,
    settings: SummarySettings,
    thinking: bool,
    prompt: SummaryPrompt,
    owner: VideoOwner,
}

/// Why a video isn't summarized now. Nothing about the video changes.
enum Skip {
    /// It shouldn't be, for now or for good.
    NotWanted(&'static str),
    /// It should be, but something outside the video stops it.
    Blocked(anyhow::Error),
}

/// Why a generation failed.
enum Failure {
    /// The server's fault: a later sync can try again.
    Server(anyhow::Error),
    /// This video didn't get a summary out of the settings as they are.
    Rejected(anyhow::Error),
}

impl From<anyhow::Error> for Failure {
    fn from(error: anyhow::Error) -> Self {
        Self::Rejected(error)
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
    clock: Arc<dyn Clock>,
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
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_repository,
            video_owner_finder,
            preferences_repository,
            video_transcript_repository,
            video_file_repository,
            llm_server_repository,
            video_summary_repository,
            clock,
        }
    }
}

pub trait VideoSummarizerApi: Send + Sync {
    /// See the `video-summaries` capability. An error fails the task, for the
    /// queue to retry. Holds no `VideoLock`: generation takes minutes.
    fn summarize(&self, video: &Video, is_last_attempt: bool) -> anyhow::Result<()>;
}

impl VideoSummarizerApi for VideoSummarizer {
    fn summarize(&self, video: &Video, is_last_attempt: bool) -> anyhow::Result<()> {
        match self.prepare(video) {
            Ok(job) => self.generate(video, &job, is_last_attempt),
            Err(Skip::NotWanted(reason)) => {
                info!(video_id = %video.id, reason, "skipped video summary");
                Ok(())
            }
            Err(Skip::Blocked(reason)) => {
                warn!(video_id = %video.id, %reason, "couldn't summarize video");
                Ok(())
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
        let url = preferences.ollama_url.context("no Ollama URL is saved")?;
        let settings = preferences.summaries;
        let Some(owner) = self.video_owner_finder.find(video)? else {
            return Err(Skip::NotWanted("the video has no channel or playlist"));
        };
        if !owner.summaries_enabled {
            return Err(Skip::NotWanted("its channel or playlist has summaries off"));
        }
        let server = self.llm_server_repository.find(&url)?;
        let model = server
            .model(settings.model.as_str())
            .with_context(|| format!("{} isn't installed on Ollama", settings.model.as_str()))?;
        let thinking = settings.thinking && model.thinking;
        let prompt = settings
            .effective_prompt()
            .validated()
            .map_err(anyhow::Error::new)?;
        Ok(SummaryJob {
            url,
            settings,
            thinking,
            prompt,
            owner,
        })
    }

    fn generate(
        &self,
        video: &Video,
        job: &SummaryJob,
        is_last_attempt: bool,
    ) -> anyhow::Result<()> {
        self.try_generate(video, job).map_err(|failure| {
            let (cause, gives_up) = match failure {
                Failure::Server(cause) => (cause, false),
                Failure::Rejected(cause) => (cause, is_last_attempt),
            };
            error!(
                video_id = %video.id,
                error = format!("{cause:#}"),
                is_last_attempt,
                gives_up,
                "failed to summarize video"
            );
            if gives_up {
                self.give_up(video);
            }
            cause
        })
    }

    /// Best effort: the error being reported matters more than this one.
    fn give_up(&self, video: &Video) {
        let unavailable = video.clone().mark_summary_unavailable(self.clock.now());
        if let Err(e) = self.video_repository.update_summary(&unavailable) {
            error!(video_id = %video.id, error = %e, "failed to give up on a summary");
        }
    }

    fn try_generate(&self, video: &Video, job: &SummaryJob) -> Result<(), Failure> {
        let started = Instant::now();
        let input = self.summary_input(video, job)?;
        info!(
            video_id = %video.id,
            title = %video.title,
            video_seconds = video.duration_seconds,
            parts = input.parts(),
            model = %job.settings.model.as_str(),
            "generating video summary"
        );
        let text = self.write_summary(video, &input, job)?;
        if self.video_repository.find(&video.id)?.is_none() {
            return Ok(());
        }
        let now = self.clock.now();
        self.video_summary_repository.save(&VideoSummary::create(
            video.id.clone(),
            text.clone(),
            &job.settings.model,
            now,
        ))?;
        self.video_repository
            .update_summary(&video.clone().mark_summary_generated(now))?;
        info!(
            video_id = %video.id,
            summary = %text.as_str(),
            duration_seconds = started.elapsed().as_secs_f64(),
            video_seconds = video.duration_seconds,
            model = %job.settings.model.as_str(),
            "video summary generated"
        );
        Ok(())
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
            return self.ask(video, job, "whole", input.whole(&job.prompt));
        }
        let answers = (0..parts)
            .map(|index| {
                let call = format!("part {} of {parts}", index + 1);
                self.ask(video, job, &call, input.part(&job.prompt, index))
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
            return self.ask(video, job, "reduce", input.reduce(&job.prompt, &answers));
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
                    let messages = input.reduce_group(&job.prompt, group);
                    self.ask(video, job, &call, messages)
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
        let context_length = job.settings.context_length;
        let reply = self
            .llm_server_repository
            .chat(
                &job.url,
                &LlmChat {
                    model: job.settings.model.as_str().to_string(),
                    system: messages.system,
                    user: messages.user,
                    context_length: context_length.value(),
                    temperature: job.settings.temperature.value(),
                    thinking: job.thinking,
                    max_output_tokens: context_length.value(),
                },
            )
            .map_err(Failure::Server)?;
        info!(
            video_id = %video.id,
            call,
            answer = %reply.content,
            input_tokens = reply.input_tokens,
            stopped_at_limit = reply.stopped_at_limit,
            duration_seconds = started.elapsed().as_secs_f64(),
            "model answered"
        );
        if let Some(refusal) = reply.refusal {
            return Err(anyhow!("Ollama refused the call: {refusal}").into());
        }
        if reply.stopped_at_limit {
            return Err(anyhow!(
                "the model ran out of context ({} tokens: {} of input, {} left to think and answer)",
                context_length.value(),
                reply.input_tokens,
                context_length.value().saturating_sub(reply.input_tokens)
            )
            .into());
        }
        SummaryText::from_answer(&reply.content).map_err(|e| anyhow::Error::new(e).into())
    }

    fn summary_input(&self, video: &Video, job: &SummaryJob) -> anyhow::Result<SummaryInput> {
        let transcript = self
            .video_transcript_repository
            .find(&video.id)?
            .context("video has no stored transcript")?;
        let filename = video
            .subtitle_filename
            .as_deref()
            .context("video has no subtitle file")?;
        let srt = self
            .video_file_repository
            .read_to_string(&job.owner.target.output_dir, filename)?
            .context("video's subtitle file is gone")?;
        let timed = TimedTranscript::from_srt(&srt);
        anyhow::ensure!(!timed.is_empty(), "video's subtitle file has no text");
        Ok(SummaryInput::new(
            video,
            &transcript.language,
            &timed,
            &job.settings.context_length,
        ))
    }
}
