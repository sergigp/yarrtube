use crate::domain::channel::ChannelHandle;
use crate::domain::services::ChannelVideoReconciler;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ChannelCreatedPayload {
    channel_id: String,
}

/// Reacts to `ChannelCreated` by running one reconcile pass for the new
/// channel, as that event's own processing (not as a separately scheduled
/// task) — mirrors `ReconcileOnPlaylistCreated`.
pub struct ReconcileOnChannelCreated {
    channel_video_reconciler: ChannelVideoReconciler,
}

impl ReconcileOnChannelCreated {
    pub fn new(channel_video_reconciler: ChannelVideoReconciler) -> Self {
        Self {
            channel_video_reconciler,
        }
    }
}

impl EventSubscriber for ReconcileOnChannelCreated {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: ChannelCreatedPayload = serde_json::from_str(payload)?;
        let Ok(channel_id) = ChannelHandle::new(payload.channel_id) else {
            return Ok(());
        };
        self.channel_video_reconciler.reconcile(channel_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, VideoLimit};
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::services::ThumbnailFetcher;
    use crate::domain::shared::Quality;
    use crate::domain::task::{ScheduledTask, Task, TaskStatus};
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, SqliteChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::SqliteChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::SqliteVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::repositories::youtube_channel_videos_repository::FakeChannelVideosRepository;
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
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
    fn it_should_reconcile_the_channel_named_in_the_payload() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let subscriber = ReconcileOnChannelCreated::new(channel_video_reconciler(
            &db,
            channel_repository,
            task_repository.clone(),
        ));

        let result = handle(&subscriber, r#"{"channel_id": "@somechannel"}"#);

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::ReconcileChannel {
                    channel_id: "@somechannel".to_string(),
                },
                fixed_timestamp() + chrono::Duration::seconds(3600),
            )]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_no_op_when_the_payload_channel_id_is_invalid() {
        let result = handle(&any_subscriber(), r#"{"channel_id": ""}"#);

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn it_should_no_op_when_the_channel_no_longer_exists() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let subscriber = ReconcileOnChannelCreated::new(channel_video_reconciler(
            &db,
            Arc::new(SqliteChannelRepository::new(db.connection())),
            task_repository.clone(),
        ));

        let result = handle(&subscriber, r#"{"channel_id": "@missing"}"#);

        assert_eq!(result, Ok(()));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    /// Builds a reconciler around the repositories a test seeds and asserts;
    /// the remaining ports (YouTube, metadata, files, thumbnails) are ones no
    /// test here observes. Events go to `db`'s outbox table.
    fn channel_video_reconciler(
        db: &TestDatabase,
        channel_repository: Arc<SqliteChannelRepository>,
        task_repository: Arc<SqliteTaskRepository>,
    ) -> ChannelVideoReconciler {
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        ChannelVideoReconciler::new(
            channel_repository,
            video_repository,
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            Arc::new(FakeChannelVideosRepository::default()),
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
    fn any_subscriber() -> ReconcileOnChannelCreated {
        let video_repository = Arc::new(SqliteVideoRepository::new(unused_connection()));
        ReconcileOnChannelCreated::new(ChannelVideoReconciler::new(
            Arc::new(SqliteChannelRepository::new(unused_connection())),
            video_repository.clone(),
            Arc::new(SqliteChannelVideoRepository::new(unused_connection())),
            Arc::new(FakeChannelVideosRepository::default()),
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

    fn channel(channel_handle: &str) -> Channel {
        Channel::create(
            ChannelHandle::new(channel_handle).unwrap(),
            "Some Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
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

    fn handle(subscriber: &ReconcileOnChannelCreated, payload: &str) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
