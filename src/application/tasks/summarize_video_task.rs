use crate::domain::services::{VideoSummarizer, VideoSummarizerApi};
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::sync::Arc;

/// Writes one video's AI summary, scheduled by the video sync
/// (`SyncVideoTask` via `VideoReconciler`) while the video needs one (see the
/// `video-summaries` capability). Best-effort: always succeeds, so the queue
/// never retries it.
pub struct SummarizeVideoTask {
    video_repository: Arc<dyn VideoRepository>,
    video_summarizer: Arc<VideoSummarizer>,
}

impl SummarizeVideoTask {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_summarizer: Arc<VideoSummarizer>,
    ) -> Self {
        Self {
            video_repository,
            video_summarizer,
        }
    }
}

impl TaskHandler for SummarizeVideoTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let video_id = VideoRecordId::new(Task::decode_summarize_video_payload(payload)?)?;
        let Some(video) = self.video_repository.find(&video_id)? else {
            return Ok(());
        };
        self.video_summarizer.summarize(&video);
        Ok(())
    }
}
