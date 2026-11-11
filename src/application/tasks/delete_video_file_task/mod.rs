use crate::domain::services::{VideoFileDeleter, VideoFileDeleterApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};
use std::path::Path;

/// Deletes one video's downloaded file, scheduled by
/// `subscribers::delete_video_file_on_video_removed_from_playlist`/
/// `..._from_channel` whenever a downloaded video is removed from its
/// tracked container.
pub struct DeleteVideoFileTask {
    video_file_deleter: VideoFileDeleter,
}

impl DeleteVideoFileTask {
    pub fn new(video_file_deleter: VideoFileDeleter) -> Self {
        Self { video_file_deleter }
    }
}

impl TaskHandler for DeleteVideoFileTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::DeleteVideoFile {
            filename,
            thumbnail_filename,
            subtitle_filename,
            output_dir,
        } = task
        else {
            return Err(unexpected_task(task));
        };
        self.video_file_deleter
            .delete_video_file(
                filename.clone(),
                thumbnail_filename.clone(),
                subtitle_filename.clone(),
                Path::new(output_dir),
            )
            .map(|()| TaskOutcome::Done)
    }
}

#[cfg(test)]
mod tests;
