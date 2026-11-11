use crate::domain::playlist::PlaylistId;
use crate::domain::services::{VideoFileDeleter, VideoFileDeleterApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};

/// Recursively deletes a deleted playlist's output directory, scheduled by
/// `subscribers::delete_playlist_files_on_playlist_deleted` whenever a
/// playlist is deleted.
pub struct DeletePlaylistFilesTask {
    video_file_deleter: VideoFileDeleter,
}

impl DeletePlaylistFilesTask {
    pub fn new(video_file_deleter: VideoFileDeleter) -> Self {
        Self { video_file_deleter }
    }
}

impl TaskHandler for DeletePlaylistFilesTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::DeletePlaylistFiles { playlist_id, path } = task else {
            return Err(unexpected_task(task));
        };
        let Ok(playlist_id) = PlaylistId::new(playlist_id.clone()) else {
            return Ok(TaskOutcome::Done);
        };
        self.video_file_deleter
            .delete_playlist_video_files(playlist_id, path.clone())
            .map(|()| TaskOutcome::Done)
    }
}

#[cfg(test)]
mod tests;
