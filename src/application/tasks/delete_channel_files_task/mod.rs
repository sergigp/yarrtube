use crate::domain::channel::ChannelHandle;
use crate::domain::services::{VideoFileDeleter, VideoFileDeleterApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};

/// Recursively deletes a deleted channel's output directory, scheduled by
/// `subscribers::delete_channel_files_on_channel_deleted` whenever a channel
/// is deleted.
pub struct DeleteChannelFilesTask {
    video_file_deleter: VideoFileDeleter,
}

impl DeleteChannelFilesTask {
    pub fn new(video_file_deleter: VideoFileDeleter) -> Self {
        Self { video_file_deleter }
    }
}

impl TaskHandler for DeleteChannelFilesTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::DeleteChannelFiles { channel_id, path } = task else {
            return Err(unexpected_task(task));
        };
        let Ok(channel_id) = ChannelHandle::new(channel_id.clone()) else {
            return Ok(TaskOutcome::Done);
        };
        self.video_file_deleter
            .delete_channel_video_files(channel_id, path.clone())
            .map(|()| TaskOutcome::Done)
    }
}

#[cfg(test)]
mod tests;
