use crate::domain::services::{VideoDownloader, VideoDownloaderApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};

/// Downloads one video via `yt-dlp`, scheduled by
/// the video sync (`SyncVideoTask` via `VideoReconciler`) whenever a video
/// that isn't downloaded yet needs it, or by a reconcile pass.
pub struct DownloadVideoTask {
    video_downloader: VideoDownloader,
}

impl DownloadVideoTask {
    pub fn new(video_downloader: VideoDownloader) -> Self {
        Self { video_downloader }
    }
}

impl TaskHandler for DownloadVideoTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::DownloadVideo { video_id } = task else {
            return Err(unexpected_task(task));
        };
        self.video_downloader
            .download(VideoId::new(video_id.clone())?)
            .map(TaskOutcome::from)
    }
}

#[cfg(test)]
mod tests;
