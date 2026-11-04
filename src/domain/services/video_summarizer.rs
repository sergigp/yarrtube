use crate::domain::llm_server::LlmChat;
use crate::domain::preferences::SummarySettings;
use crate::domain::services::{VideoOwnerFinder, VideoOwnerFinderApi};
use crate::domain::video::{Video, VideoOwner};
use crate::domain::video_summary::{SummaryInput, SummaryText, VideoSummary};
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
        let settings = preferences.summaries;
        let url = preferences.ollama_url.context("no Ollama URL is saved")?;
        let owner = self
            .video_owner_finder
            .find(video)?
            .context("video has no owner")?;
        let prompt = settings.effective_prompt();
        let input = self.summary_input(video, &owner, &settings)?;
        let messages = input.whole(&prompt);
        let chat = LlmChat {
            model: settings.model.as_str().to_string(),
            system: messages.system,
            user: messages.user,
            context_length: settings.context_length.value(),
            temperature: settings.temperature.value(),
            thinking: settings.thinking,
            max_output_tokens: settings.context_length.output_budget_tokens(),
        };
        let reply = self.llm_server_repository.chat(&url, &chat)?;
        let text = SummaryText::from_answer(&reply.content)?;
        let now = self.clock.now();
        self.video_summary_repository.save(&VideoSummary::create(
            video.id.clone(),
            text,
            &settings.model,
            now,
        ))?;
        self.video_repository
            .update_summary(&video.clone().mark_summary_generated(now))
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
