use crate::domain::services::{VideoSummarizer, VideoSummarizerApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};
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
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::SummarizeVideo { video_id } = task else {
            return Err(unexpected_task(task));
        };
        let video_id = VideoId::new(video_id.clone())?;
        let Some(video) = self.video_repository.find(&video_id)? else {
            return Ok(TaskOutcome::Done);
        };
        self.video_summarizer
            .summarize(&video)
            .map(TaskOutcome::from)
    }
}

#[cfg(test)]
mod tests;
