use crate::domain::llm_server::{LlmChat, OllamaUrl};
use crate::domain::preferences::{SummaryPrompt, SummarySettings};
use crate::domain::services::{VideoOwnerFinder, VideoOwnerFinderApi};
use crate::domain::video::{Video, VideoOwner};
use crate::domain::video_summary::{SummaryInput, SummaryMessages, SummaryText, VideoSummary};
use crate::domain::video_transcript::TimedTranscript;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::ollama_llm_server_repository::LlmServerRepository;
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::sqlite_video_summary_repository::VideoSummaryRepository;
use crate::infrastructure::repositories::sqlite_video_transcript_repository::VideoTranscriptRepository;
use crate::infrastructure::shared::system_clock::Clock;
use anyhow::Context;
use std::sync::Arc;
use tracing::warn;

/// What every call of one generation shares.
struct SummaryJob {
    url: OllamaUrl,
    settings: SummarySettings,
    thinking: bool,
    prompt: SummaryPrompt,
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
    /// Best effort: see the `video-summaries` capability. Never returns an
    /// error to its caller. Holds no `VideoLock`: generation takes minutes.
    fn summarize(&self, video: &Video);
}

impl VideoSummarizerApi for VideoSummarizer {
    fn summarize(&self, video: &Video) {
        if let Err(e) = self.generate(video) {
            warn!(video_id = %video.id, error = %e, "failed to summarize video");
        }
    }
}

impl VideoSummarizer {
    fn generate(&self, video: &Video) -> anyhow::Result<()> {
        let preferences = self.preferences_repository.find()?.unwrap_or_default();
        let url = preferences.ollama_url.context("no Ollama URL is saved")?;
        let settings = preferences.summaries;
        let owner = self
            .video_owner_finder
            .find(video)?
            .context("video has no owner")?;
        let server = self.llm_server_repository.find(&url)?;
        let thinking = settings.thinking
            && server
                .model(settings.model.as_str())
                .is_some_and(|model| model.thinking);
        let prompt = settings.effective_prompt();
        let input = self.summary_input(video, &owner, &settings)?;
        let job = SummaryJob {
            url,
            settings,
            thinking,
            prompt,
        };
        let text = self.write_summary(&input, &job)?;
        let now = self.clock.now();
        self.video_summary_repository.save(&VideoSummary::create(
            video.id.clone(),
            text,
            &job.settings.model,
            now,
        ))?;
        self.video_repository
            .update_summary(&video.clone().mark_summary_generated(now))
    }

    /// One call for a whole transcript; for one in parts, a call per part
    /// and one over their answers. Stops at the first failed call.
    fn write_summary(&self, input: &SummaryInput, job: &SummaryJob) -> anyhow::Result<SummaryText> {
        if input.parts() == 1 {
            return self.ask(job, input.whole(&job.prompt));
        }
        let answers = (0..input.parts())
            .map(|index| self.ask(job, input.part(&job.prompt, index)))
            .collect::<anyhow::Result<Vec<_>>>()?;
        self.ask(job, input.reduce(&job.prompt, &answers))
    }

    fn ask(&self, job: &SummaryJob, messages: SummaryMessages) -> anyhow::Result<SummaryText> {
        let context_length = job.settings.context_length;
        let reply = self.llm_server_repository.chat(
            &job.url,
            &LlmChat {
                model: job.settings.model.as_str().to_string(),
                system: messages.system,
                user: messages.user,
                context_length: context_length.value(),
                temperature: job.settings.temperature.value(),
                thinking: job.thinking,
                max_output_tokens: context_length.output_budget_tokens(),
            },
        )?;
        Ok(SummaryText::from_answer(&reply.content)?)
    }

    fn summary_input(
        &self,
        video: &Video,
        owner: &VideoOwner,
        settings: &SummarySettings,
    ) -> anyhow::Result<SummaryInput> {
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
            .read_to_string(&owner.target.output_dir, filename)?
            .context("video's subtitle file is gone")?;
        Ok(SummaryInput::new(
            video,
            &transcript.language,
            &TimedTranscript::from_srt(&srt),
            &settings.context_length,
        ))
    }
}
