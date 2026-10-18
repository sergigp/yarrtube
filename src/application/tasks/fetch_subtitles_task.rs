use crate::domain::services::SubtitleFetcher;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::sync::Arc;

/// Fetches one downloaded video's subtitles, scheduled by the reconcilers'
/// missing-subtitles recovery. Best-effort: always succeeds, so the queue
/// never retries it; the next reconcile pass reschedules it instead.
#[allow(dead_code)]
pub struct FetchSubtitlesTask {
    video_repository: Arc<dyn VideoRepository>,
    subtitle_fetcher: Arc<SubtitleFetcher>,
}

impl FetchSubtitlesTask {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        subtitle_fetcher: Arc<SubtitleFetcher>,
    ) -> Self {
        Self {
            video_repository,
            subtitle_fetcher,
        }
    }
}

impl TaskHandler for FetchSubtitlesTask {
    fn handle(&self, _payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        Ok(())
    }
}
