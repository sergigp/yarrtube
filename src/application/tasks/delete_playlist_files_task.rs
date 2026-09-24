use crate::domain::services::VideoFileDeleter;
use crate::domain::shared::PlaylistId;
use crate::domain::task::Task;
use crate::infrastructure::repositories::task_handler::TaskHandler;

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
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let (playlist_id, path) = Task::decode_delete_playlist_files_payload(payload)?;
        let Ok(playlist_id) = PlaylistId::new(playlist_id) else {
            return Ok(());
        };
        self.video_file_deleter
            .delete_playlist_video_files(playlist_id, path)
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

        let result = run(&task, &payload_for("PL1", "music/chill"));

        assert_eq!(result, Ok(()));
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

        let result = run(&task, &payload_for("", "music/chill"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let task = DeletePlaylistFilesTask::new(VideoFileDeleter::new(
            video_file_repository.clone(),
            "/videos",
        ));

        let result = run(&task, "not json");

        assert_eq!(
            result,
            Err(
                "invalid delete_playlist_files payload: expected ident at line 1 column 2"
                    .to_string()
            )
        );
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            Vec::<PathBuf>::new()
        );
    }

    fn payload_for(playlist_id: &str, path: &str) -> String {
        Task::DeletePlaylistFiles {
            playlist_id: playlist_id.to_string(),
            path: path.to_string(),
        }
        .payload()
        .to_string()
    }

    fn run(task: &DeletePlaylistFilesTask, payload: &str) -> Result<(), String> {
        task.handle(payload, false).map_err(|e| e.to_string())
    }
}
