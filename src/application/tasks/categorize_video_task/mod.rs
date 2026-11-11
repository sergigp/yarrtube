use crate::domain::services::{VideoCategorizer, VideoCategorizerApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};
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
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::CategorizeVideo { video_id } = task else {
            return Err(unexpected_task(task));
        };
        let video_id = VideoId::new(video_id.clone())?;
        let Some(video) = self.video_repository.find(&video_id)? else {
            return Ok(TaskOutcome::Done);
        };
        self.video_categorizer
            .categorize(&video)
            .map(TaskOutcome::from)
    }
}

#[cfg(test)]
mod tests;
