use crate::domain::services::{ThumbnailFetcher, ThumbnailFetcherApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};
use std::sync::Arc;

/// Fetches one video's thumbnail ahead of its download, scheduled by
/// the video sync (`SyncVideoTask` via `VideoReconciler`) and by the
/// reconcilers' missing-thumbnail recovery.
/// Best-effort: always succeeds, so the queue never retries a video that
/// simply has no thumbnail. The next reconcile pass reschedules it instead.
pub struct FetchThumbnailTask {
    thumbnail_fetcher: Arc<ThumbnailFetcher>,
}

impl FetchThumbnailTask {
    pub fn new(thumbnail_fetcher: Arc<ThumbnailFetcher>) -> Self {
        Self { thumbnail_fetcher }
    }
}

impl TaskHandler for FetchThumbnailTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::FetchThumbnail { video_id } = task else {
            return Err(unexpected_task(task));
        };
        self.thumbnail_fetcher
            .fetch(&VideoId::new(video_id.clone())?)
            .map(TaskOutcome::from)
    }
}

#[cfg(test)]
mod tests;
