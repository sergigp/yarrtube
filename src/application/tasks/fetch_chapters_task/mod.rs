use crate::domain::services::{ChapterFetcher, ChapterFetcherApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};
use std::sync::Arc;

/// Fetches one downloaded video's chapters, scheduled by the video sync
/// (`SyncVideoTask` via `VideoReconciler`) while its chapters are unsettled and
/// due (see the `late-video-data` capability). Best-effort: always succeeds,
/// so the queue never retries it.
pub struct FetchChaptersTask {
    video_repository: Arc<dyn VideoRepository>,
    chapter_fetcher: Arc<ChapterFetcher>,
}

impl FetchChaptersTask {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        chapter_fetcher: Arc<ChapterFetcher>,
    ) -> Self {
        Self {
            video_repository,
            chapter_fetcher,
        }
    }
}

impl TaskHandler for FetchChaptersTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::FetchChapters { video_id } = task else {
            return Err(unexpected_task(task));
        };
        let video_id = VideoId::new(video_id.clone())?;
        match self.video_repository.find(&video_id)? {
            Some(video) => self.chapter_fetcher.fetch(&video).map(TaskOutcome::from),
            None => Ok(TaskOutcome::Done),
        }
    }
}

#[cfg(test)]
mod tests;
