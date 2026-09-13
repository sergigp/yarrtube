use crate::domain::shared::PlaylistId;
use crate::domain::task::Task;
use crate::domain::video::VideoService;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Recursively deletes a deleted playlist's output directory, scheduled by
/// `subscribers::delete_playlist_files_on_playlist_deleted` whenever a
/// playlist is deleted.
pub struct DeletePlaylistFilesTask {
    video_service: VideoService,
}

impl DeletePlaylistFilesTask {
    pub fn new(video_service: VideoService) -> Self {
        Self { video_service }
    }
}

impl TaskHandler for DeletePlaylistFilesTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let (playlist_id, path) = Task::decode_delete_playlist_files_payload(payload)?;
        let Ok(playlist_id) = PlaylistId::new(playlist_id) else {
            return Ok(());
        };
        self.video_service.delete_playlist_files(playlist_id, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::FakePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::repositories::youtube_video_repository::FakeYoutubeVideoRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

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
        let video_service = VideoService::new(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeVideoRepository::default()),
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeYoutubeVideoRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            video_file_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );

        (
            DeletePlaylistFilesTask::new(video_service),
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
