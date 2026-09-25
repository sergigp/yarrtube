use crate::domain::services::{PlaylistVideoReconciler, PlaylistVideoReconcilerApi};
use crate::domain::shared::PlaylistId;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PlaylistCreatedPayload {
    playlist_id: String,
}

/// Reacts to `PlaylistCreated` by running one reconcile pass for the new
/// playlist, as that event's own processing (not as a separately scheduled
/// task).
pub struct ReconcileOnPlaylistCreated {
    playlist_video_reconciler: PlaylistVideoReconciler,
}

impl ReconcileOnPlaylistCreated {
    pub fn new(playlist_video_reconciler: PlaylistVideoReconciler) -> Self {
        Self {
            playlist_video_reconciler,
        }
    }
}

impl EventSubscriber for ReconcileOnPlaylistCreated {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: PlaylistCreatedPayload = serde_json::from_str(payload)?;
        let Ok(playlist_id) = PlaylistId::new(payload.playlist_id) else {
            return Ok(());
        };
        self.playlist_video_reconciler.reconcile(playlist_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::services::ThumbnailFetcher;
    use crate::domain::shared::Quality;
    use crate::domain::task::{ScheduledTask, Task, TaskStatus};
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        PlaylistRepository, SqlitePlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::SqliteVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    #[test]
    fn it_should_reconcile_the_playlist() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let subscriber = ReconcileOnPlaylistCreated::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            task_repository.clone(),
        ));

        let result = handle(&subscriber, r#"{"playlist_id": "PL1"}"#);

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::ReconcilePlaylist {
                    playlist_id: "PL1".to_string(),
                },
                fixed_timestamp() + chrono::Duration::seconds(3600),
            )]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_if_invalid_playlist_id_provided() {
        let result = handle(&any_subscriber(), r#"{"playlist_id": ""}"#);

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn it_should_skip_if_playlist_is_gone() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let subscriber = ReconcileOnPlaylistCreated::new(playlist_video_reconciler(
            &db,
            Arc::new(SqlitePlaylistRepository::new(db.connection())),
            task_repository.clone(),
        ));

        let result = handle(&subscriber, r#"{"playlist_id": "PL404"}"#);

        assert_eq!(result, Ok(()));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    /// Builds a reconciler around the repositories a test seeds and asserts;
    /// the remaining ports (YouTube, metadata, files, thumbnails) are ones no
    /// test here observes. Events go to `db`'s outbox table.
    fn playlist_video_reconciler(
        db: &TestDatabase,
        playlist_repository: Arc<SqlitePlaylistRepository>,
        task_repository: Arc<SqliteTaskRepository>,
    ) -> PlaylistVideoReconciler {
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        PlaylistVideoReconciler::new(
            playlist_repository,
            video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection())),
            Arc::new(FakeYoutubePlaylistItemsRepository {
                videos: Mutex::new(Vec::new()),
            }),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
            event_publisher(db),
            task_repository,
            Arc::new(FakeVideoFileRepository::default()),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        )
    }

    /// A subscriber for tests whose payload is rejected before reaching the
    /// reconciler. Its repositories sit on an unmigrated in-memory database,
    /// so a payload that wrongly got through would fail loudly instead of
    /// passing.
    fn any_subscriber() -> ReconcileOnPlaylistCreated {
        let video_repository = Arc::new(SqliteVideoRepository::new(unused_connection()));
        ReconcileOnPlaylistCreated::new(PlaylistVideoReconciler::new(
            Arc::new(SqlitePlaylistRepository::new(unused_connection())),
            video_repository.clone(),
            Arc::new(SqlitePlaylistVideoRepository::new(unused_connection())),
            Arc::new(FakeYoutubePlaylistItemsRepository {
                videos: Mutex::new(Vec::new()),
            }),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(unused_connection())),
            Arc::new(SqliteEventPublisher::new(
                Arc::new(Mutex::new(unused_connection())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteTaskRepository::new(
                Arc::new(Mutex::new(unused_connection())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(ThumbnailFetcher::new(
                video_repository,
                Arc::new(FakeVideoDownloaderRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        ))
    }

    fn playlist(id: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new("My Playlist").unwrap(),
            PlaylistPath::new("my-playlist").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            fixed_timestamp(),
        )
    }

    fn event_publisher(db: &TestDatabase) -> Arc<SqliteEventPublisher> {
        Arc::new(SqliteEventPublisher::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ))
    }

    fn unused_connection() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    fn pending_task(id: i64, task: &Task, run_at: DateTime<Utc>) -> ScheduledTask {
        ScheduledTask {
            id,
            task_type: task.task_type().to_string(),
            payload: task.payload().to_string(),
            status: TaskStatus::Pending,
            retries: 0,
            run_at,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn handle(subscriber: &ReconcileOnPlaylistCreated, payload: &str) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
