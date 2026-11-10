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
mod tests {
    use super::*;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn it_should_delete_the_playlist_directory() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let task = DeletePlaylistFilesTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, &task_for("PL1", "music/chill"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from("/videos/music/chill")]
        );
    }

    #[test]
    fn it_should_skip_if_invalid_playlist_id_provided() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let task = DeletePlaylistFilesTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, &task_for("", "music/chill"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn it_should_fail_if_handed_another_task() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let task = DeletePlaylistFilesTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, &Task::UpdateYtdlp);

        assert_eq!(
            result,
            Err("unexpected update_ytdlp task for this handler".to_string())
        );
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            Vec::<PathBuf>::new()
        );
    }

    fn task_for(playlist_id: &str, path: &str) -> Task {
        Task::DeletePlaylistFiles {
            playlist_id: playlist_id.to_string(),
            path: path.to_string(),
        }
    }

    fn run(task: &DeletePlaylistFilesTask, queued: &Task) -> Result<TaskOutcome, String> {
        task.handle(queued).map_err(|e| e.to_string())
    }
}
