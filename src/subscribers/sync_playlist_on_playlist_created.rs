use crate::domain::shared::PlaylistId;
use crate::domain::video::VideoService;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PlaylistCreatedPayload {
    playlist_id: String,
}

/// Reacts to `PlaylistCreated` by syncing the new playlist's videos, as that
/// event's own processing (not as a separately scheduled task).
pub struct SyncPlaylistOnPlaylistCreated {
    video_service: VideoService,
}

impl SyncPlaylistOnPlaylistCreated {
    pub fn new(video_service: VideoService) -> Self {
        Self { video_service }
    }
}

impl EventSubscriber for SyncPlaylistOnPlaylistCreated {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: PlaylistCreatedPayload = serde_json::from_str(payload)?;
        let Ok(playlist_id) = PlaylistId::new(payload.playlist_id) else {
            return Ok(());
        };
        self.video_service.sync_playlist_videos(playlist_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::playlist::{Playlist, PlaylistName, Quality};
    use crate::domain::task::Task;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    #[test]
    fn it_should_no_op_when_the_payload_playlist_id_is_invalid() {
        let task_repository = Arc::new(FakeTaskRepository::default());
        let video_service = VideoService::new(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeVideoRepository::default()),
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            task_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(DateTime::<Utc>::from_timestamp(0, 0).unwrap())),
            3600,
            "/videos",
        );
        let subscriber = SyncPlaylistOnPlaylistCreated::new(video_service);

        subscriber.handle(r#"{"playlist_id": ""}"#).unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_sync_the_playlist_named_in_the_payload() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        playlist_repository
            .insert(&Playlist::create(
                PlaylistId::new("PL1").unwrap(),
                PlaylistName::new("My Playlist").unwrap(),
                Quality::High,
                now,
            ))
            .unwrap();
        let task_repository = Arc::new(FakeTaskRepository::default());

        let video_service = VideoService::new(
            playlist_repository,
            Arc::new(FakeVideoRepository::default()),
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            task_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(DateTime::<Utc>::from_timestamp(0, 0).unwrap())),
            3600,
            "/videos",
        );
        let subscriber = SyncPlaylistOnPlaylistCreated::new(video_service);

        subscriber.handle(r#"{"playlist_id": "PL1"}"#).unwrap();

        assert_eq!(
            task_repository
                .scheduled
                .lock()
                .unwrap()
                .iter()
                .map(|(task, _run_at)| task.clone())
                .collect::<Vec<_>>(),
            vec![Task::SyncPlaylist {
                playlist_id: "PL1".to_string()
            }]
        );
    }
}
