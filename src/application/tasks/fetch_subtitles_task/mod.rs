use crate::domain::services::{SubtitleFetcher, SubtitleFetcherApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};
use std::sync::Arc;

/// Fetches one downloaded video's subtitles, scheduled by
/// the video sync (`SyncVideoTask` via `VideoReconciler`) after a download
/// and by the reconcilers' missing-subtitles recovery. Best-effort: always succeeds, so
/// the queue never retries it; the next reconcile pass reschedules it instead.
pub struct FetchSubtitlesTask {
    subtitle_fetcher: Arc<SubtitleFetcher>,
}

impl FetchSubtitlesTask {
    pub fn new(subtitle_fetcher: Arc<SubtitleFetcher>) -> Self {
        Self { subtitle_fetcher }
    }
}

impl TaskHandler for FetchSubtitlesTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::FetchSubtitles { video_id } = task else {
            return Err(unexpected_task(task));
        };
        self.subtitle_fetcher
            .fetch(&VideoId::new(video_id.clone())?)
            .map(TaskOutcome::from)
    }
}

#[cfg(test)]
mod tests;
