use crate::domain::services::VideoOwnerFinder;
use crate::domain::video::Video;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::ollama_llm_server_repository::LlmServerRepository;
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::sqlite_video_summary_repository::VideoSummaryRepository;
use crate::infrastructure::repositories::sqlite_video_transcript_repository::VideoTranscriptRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

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
    fn summarize(&self, _video: &Video) {}
}
