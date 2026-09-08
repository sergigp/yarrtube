use crate::domain::playlist::{PlaylistService, YoutubePlaylistId};
use crate::domain::task::Task;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Runs every subsequent sync for a playlist (the first one is triggered by
/// `subscribers::sync_playlist_on_playlist_created` instead).
pub struct SyncPlaylistTask {
    playlist_service: PlaylistService,
}

impl SyncPlaylistTask {
    pub fn new(playlist_service: PlaylistService) -> Self {
        Self { playlist_service }
    }
}

impl TaskHandler for SyncPlaylistTask {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let playlist_id = Task::decode_sync_playlist_payload(payload)?;
        let Ok(playlist_id) = YoutubePlaylistId::new(playlist_id) else {
            return Ok(());
        };
        self.playlist_service.sync_playlist(playlist_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::DomainEvent;
    use crate::domain::playlist::{Playlist, PlaylistName, YoutubePlaylistId};
    use crate::domain::video::{Video, YoutubeVideoId};
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
        fn find(&self, id: &YoutubePlaylistId) -> anyhow::Result<Option<Playlist>> {
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
            _id: &YoutubePlaylistId,
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
        fn exists(&self, _id: &YoutubePlaylistId) -> anyhow::Result<bool> {
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

    #[derive(Default)]
    struct FakeVideoRepository {
        upserted: Mutex<Vec<String>>,
    }

    impl VideoRepository for FakeVideoRepository {
        fn upsert(&self, video: &Video) -> anyhow::Result<()> {
            self.upserted
                .lock()
                .unwrap()
                .push(video.playlist_id.as_str().to_string());
            Ok(())
        }

        fn list_for_playlist(
            &self,
            _playlist_id: &YoutubePlaylistId,
        ) -> anyhow::Result<Vec<Video>> {
            Ok(Vec::new())
        }

        fn delete_not_in(
            &self,
            _playlist_id: &YoutubePlaylistId,
            _current_ids: &[YoutubeVideoId],
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct FakeYoutubePlaylistItemsRepository;

    impl YoutubePlaylistItemsRepository for FakeYoutubePlaylistItemsRepository {
        fn list_current_videos(
            &self,
            _playlist_id: &YoutubePlaylistId,
        ) -> anyhow::Result<Vec<PlaylistVideo>> {
            Ok(vec![PlaylistVideo {
                youtube_video_id: "vid1".to_string(),
                title: "One".to_string(),
            }])
        }
    }

    struct NoopTaskRepository;

    impl TaskRepository for NoopTaskRepository {
        fn schedule(&self, _task: &Task, _run_at: DateTime<Utc>) -> anyhow::Result<()> {
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
    fn it_should_sync_the_playlist_named_in_the_task_payload() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        playlist_repository
            .insert_with_event(
                &Playlist::create(
                    YoutubePlaylistId::new("PL1").unwrap(),
                    PlaylistName::new("My Playlist").unwrap(),
                    now,
                ),
                &DomainEvent::PlaylistCreated {
                    playlist_id: "PL1".to_string(),
                },
                now,
            )
            .unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());

        let playlist_service = PlaylistService::new(
            playlist_repository,
            Arc::new(NoopYoutubePlaylistRepository),
            Arc::new(FixedClock(DateTime::<Utc>::from_timestamp(0, 0).unwrap())),
            Arc::new(NoopEventPublisher),
            video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository),
            Arc::new(NoopTaskRepository),
            3600,
        );
        let handler = SyncPlaylistTask::new(playlist_service);

        handler
            .handle(
                &Task::SyncPlaylist {
                    playlist_id: "PL1".to_string(),
                }
                .payload()
                .to_string(),
            )
            .unwrap();

        assert_eq!(*video_repository.upserted.lock().unwrap(), vec!["PL1"]);
    }
}
