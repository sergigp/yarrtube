use crate::domain::services::{VideoCategorizer, VideoCategorizerApi};
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::sync::Arc;

/// Asks the AI which of its pending categories one video fits, scheduled by
/// the video sync (`SyncVideoTask` via `VideoReconciler`) while any is
/// pending (see the `video-categorization` capability). Fails when a call
/// was rejected, so the queue retries the categories still pending.
pub struct CategorizeVideoTask {
    video_repository: Arc<dyn VideoRepository>,
    video_categorizer: Arc<VideoCategorizer>,
}

impl CategorizeVideoTask {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_categorizer: Arc<VideoCategorizer>,
    ) -> Self {
        Self {
            video_repository,
            video_categorizer,
        }
    }
}

impl TaskHandler for CategorizeVideoTask {
    fn handle(&self, payload: &str, is_last_attempt: bool) -> anyhow::Result<()> {
        let video_id = VideoRecordId::new(Task::decode_categorize_video_payload(payload)?)?;
        let Some(video) = self.video_repository.find(&video_id)? else {
            return Ok(());
        };
        self.video_categorizer.categorize(&video, is_last_attempt)
    }
}
