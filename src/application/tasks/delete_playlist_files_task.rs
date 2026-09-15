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
    use std::sync::Arc;

    fn payload_for(playlist_id: &str, path: &str) -> String {
        Task::DeletePlaylistFiles {
            playlist_id: playlist_id.to_string(),
            path: path.to_string(),
        }
        .payload()
        .to_string()
    }

    fn handler() -> (DeletePlaylistFilesTask, Arc<FakeVideoFileRepository>) {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_file_deleter = VideoFileDeleter::new(video_file_repository.clone(), "/videos");

        (
            DeletePlaylistFilesTask::new(video_file_deleter),
            video_file_repository,
        )
    }

    #[test]
    fn it_should_recursively_delete_the_playlist_output_directory() {
        let (handler, video_file_repository) = handler();

        handler
            .handle(&payload_for("PL1", "music/chill"), false)
            .unwrap();

        let deleted = video_file_repository.deleted_dirs.lock().unwrap();
        assert_eq!(
            *deleted,
            vec![std::path::PathBuf::from("/videos/music/chill")]
        );
    }

    #[test]
    fn it_should_no_op_when_the_payload_playlist_id_is_invalid() {
        let (handler, video_file_repository) = handler();

        handler
            .handle(&payload_for("", "music/chill"), false)
            .unwrap();

        assert!(
            video_file_repository
                .deleted_dirs
                .lock()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let (handler, _video_file_repository) = handler();

        assert!(handler.handle("not json", false).is_err());
    }
}
