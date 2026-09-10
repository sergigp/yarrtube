use crate::domain::shared::{PlaylistId, VideoId};
use crate::domain::task::Task;
use crate::domain::video::VideoService;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Deletes one video's downloaded file, scheduled by
/// `subscribers::delete_video_file_on_video_deleted` whenever a downloaded
/// video is removed from its tracked playlist.
pub struct DeleteVideoFileTask {
    video_service: VideoService,
}

impl DeleteVideoFileTask {
    pub fn new(video_service: VideoService) -> Self {
        Self { video_service }
    }
}

impl TaskHandler for DeleteVideoFileTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let (playlist_id, video_id, title) = Task::decode_delete_video_file_payload(payload)?;
        let Ok(playlist_id) = PlaylistId::new(playlist_id) else {
            return Ok(());
        };
        let Ok(video_id) = VideoId::new(video_id) else {
            return Ok(());
        };
        self.video_service
            .delete_video_file(playlist_id, video_id, &title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::playlist::{Playlist, PlaylistName, Quality};
    use crate::infrastructure::repositories::sqlite_event_repository::FakeEventPublisher;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::system_clock::FixedClock;
    use crate::infrastructure::repositories::video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn payload_for(playlist_id: &str, video_id: &str, title: &str) -> String {
        Task::DeleteVideoFile {
            playlist_id: playlist_id.to_string(),
            video_id: video_id.to_string(),
            title: title.to_string(),
        }
        .payload()
        .to_string()
    }

    fn handler_with_seeded_playlist(
        video_file_repository: FakeVideoFileRepository,
    ) -> (DeleteVideoFileTask, Arc<FakeVideoFileRepository>) {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert(&Playlist::create(
                PlaylistId::new("PL1").unwrap(),
                PlaylistName::new("My Playlist").unwrap(),
                Quality::High,
                fixed_timestamp(),
            ))
            .unwrap();
        let video_file_repository = Arc::new(video_file_repository);

        let video_service = VideoService::new(
            playlist_repository,
            Arc::new(FakeVideoRepository::default()),
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            video_file_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );

        (
            DeleteVideoFileTask::new(video_service),
            video_file_repository,
        )
    }

    #[test]
    fn it_should_delete_the_video_file() {
        let (handler, video_file_repository) =
            handler_with_seeded_playlist(FakeVideoFileRepository::new(Ok(true)));

        handler
            .handle(&payload_for("PL1", "vid1", "My Video"), false)
            .unwrap();

        assert_eq!(video_file_repository.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn it_should_no_op_when_the_payload_ids_are_invalid() {
        let (handler, video_file_repository) =
            handler_with_seeded_playlist(FakeVideoFileRepository::new(Ok(true)));

        handler
            .handle(&payload_for("", "vid1", "My Video"), false)
            .unwrap();

        assert!(video_file_repository.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let (handler, _video_file_repository) =
            handler_with_seeded_playlist(FakeVideoFileRepository::new(Ok(true)));

        assert!(handler.handle("not json", false).is_err());
    }
}
