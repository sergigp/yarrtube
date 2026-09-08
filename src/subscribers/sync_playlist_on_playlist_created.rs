use crate::domain::playlist::PlaylistService;
use crate::domain::shared::PlaylistId;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PlaylistCreatedPayload {
    playlist_id: String,
}

/// Reacts to `PlaylistCreated` by syncing the new playlist's videos, as that
/// event's own processing (not as a separately scheduled task).
pub struct SyncPlaylistOnPlaylistCreated {
    playlist_service: PlaylistService,
}

impl SyncPlaylistOnPlaylistCreated {
    pub fn new(playlist_service: PlaylistService) -> Self {
        Self { playlist_service }
    }
}

impl EventSubscriber for SyncPlaylistOnPlaylistCreated {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: PlaylistCreatedPayload = serde_json::from_str(payload)?;
        let Ok(playlist_id) = PlaylistId::new(payload.playlist_id) else {
            return Ok(());
        };
        self.playlist_service.sync_playlist(playlist_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::DomainEvent;
    use crate::domain::playlist::{Playlist, PlaylistName};
    use crate::domain::shared::VideoId;
    use crate::domain::task::Task;
    use crate::domain::video::Video;
    use crate::infrastructure::repositories::sqlite_event_repository::EventPublisher;
    use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::{
        PersistedTask, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
    use crate::infrastructure::repositories::system_clock::Clock;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::{
        PlaylistVideo, YoutubePlaylistItemsRepository,
    };
    use crate::infrastructure::repositories::youtube_playlist_repository::YoutubePlaylistRepository;
    use chrono::{DateTime, Utc};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakePlaylistRepository {
        playlists: Mutex<Vec<Playlist>>,
    }

    impl PlaylistRepository for FakePlaylistRepository {
        fn find(&self, id: &PlaylistId) -> anyhow::Result<Option<Playlist>> {
            Ok(self
                .playlists
                .lock()
                .unwrap()
                .iter()
                .find(|p| p.id == *id)
                .cloned())
        }

        fn insert_with_event(
            &self,
            playlist: &Playlist,
            _event: &DomainEvent,
            _now: DateTime<Utc>,
        ) -> anyhow::Result<()> {
            self.playlists.lock().unwrap().push(playlist.clone());
            Ok(())
        }

        fn delete_with_event(
            &self,
            _id: &PlaylistId,
            _event: &DomainEvent,
            _now: DateTime<Utc>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        fn list(&self) -> anyhow::Result<Vec<Playlist>> {
            Ok(self.playlists.lock().unwrap().clone())
        }
    }

    struct NoopYoutubePlaylistRepository;

    impl YoutubePlaylistRepository for NoopYoutubePlaylistRepository {
        fn exists(&self, _id: &PlaylistId) -> anyhow::Result<bool> {
            Ok(true)
        }
    }

    struct FixedClock(DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    struct NoopEventPublisher;

    impl EventPublisher for NoopEventPublisher {
        fn publish(&self, _event: &DomainEvent) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct NoopVideoRepository;

    impl VideoRepository for NoopVideoRepository {
        fn upsert(&self, _video: &Video) -> anyhow::Result<()> {
            Ok(())
        }

        fn list_for_playlist(&self, _playlist_id: &PlaylistId) -> anyhow::Result<Vec<Video>> {
            Ok(Vec::new())
        }

        fn delete_not_in(
            &self,
            _playlist_id: &PlaylistId,
            _current_ids: &[VideoId],
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct NoopYoutubePlaylistItemsRepository;

    impl YoutubePlaylistItemsRepository for NoopYoutubePlaylistItemsRepository {
        fn list_current_videos(
            &self,
            _playlist_id: &PlaylistId,
        ) -> anyhow::Result<Vec<PlaylistVideo>> {
            Ok(Vec::new())
        }
    }

    #[derive(Default)]
    struct FakeTaskRepository {
        scheduled: Mutex<Vec<String>>,
    }

    impl TaskRepository for FakeTaskRepository {
        fn schedule(&self, task: &Task, _run_at: DateTime<Utc>) -> anyhow::Result<()> {
            let Task::SyncPlaylist { playlist_id } = task;
            self.scheduled.lock().unwrap().push(playlist_id.clone());
            Ok(())
        }

        fn list_eligible(&self) -> anyhow::Result<Vec<PersistedTask>> {
            Ok(Vec::new())
        }

        fn mark_running(&self, _id: i64) -> anyhow::Result<()> {
            Ok(())
        }

        fn mark_done(&self, _id: i64) -> anyhow::Result<()> {
            Ok(())
        }

        fn mark_failed_or_retry(&self, _id: i64, _error: &str) -> anyhow::Result<()> {
            Ok(())
        }

        fn recover_running(&self) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn it_should_sync_the_playlist_named_in_the_payload() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        playlist_repository
            .insert_with_event(
                &Playlist::create(
                    PlaylistId::new("PL1").unwrap(),
                    PlaylistName::new("My Playlist").unwrap(),
                    now,
                ),
                &DomainEvent::PlaylistCreated {
                    playlist_id: "PL1".to_string(),
                },
                now,
            )
            .unwrap();
        let task_repository = Arc::new(FakeTaskRepository::default());

        let playlist_service = PlaylistService::new(
            playlist_repository,
            Arc::new(NoopYoutubePlaylistRepository),
            Arc::new(FixedClock(DateTime::<Utc>::from_timestamp(0, 0).unwrap())),
            Arc::new(NoopEventPublisher),
            Arc::new(NoopVideoRepository),
            Arc::new(NoopYoutubePlaylistItemsRepository),
            task_repository.clone(),
            3600,
        );
        let subscriber = SyncPlaylistOnPlaylistCreated::new(playlist_service);

        subscriber.handle(r#"{"playlist_id": "PL1"}"#).unwrap();

        assert_eq!(*task_repository.scheduled.lock().unwrap(), vec!["PL1"]);
    }
}
