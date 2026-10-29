use crate::domain::services::plex_label_syncer::AFTER_DOWNLOAD_DELAY;
use crate::domain::services::{PlexLabelSyncer, PlexLabelSyncerApi};
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

/// The only field this subscriber reads from `VideoDownloaded`.
#[derive(Debug, Deserialize)]
struct VideoDownloadedPayload {
    video_id: String,
}

/// Reacts to `VideoDownloaded` by marking the record's Plex labels stale
/// and requesting a label sync in a minute, once Plex has imported the
/// episode. Registered only when the Plex integration is configured.
pub struct SyncPlexLabelsOnVideoDownloaded {
    plex_label_syncer: PlexLabelSyncer,
}

impl SyncPlexLabelsOnVideoDownloaded {
    pub fn new(plex_label_syncer: PlexLabelSyncer) -> Self {
        Self { plex_label_syncer }
    }
}

impl EventSubscriber for SyncPlexLabelsOnVideoDownloaded {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoDownloadedPayload = serde_json::from_str(payload)?;
        let video_id = VideoRecordId::new(payload.video_id)?;
        self.plex_label_syncer
            .mark(&[video_id], AFTER_DOWNLOAD_DELAY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plex_label::EpisodeLabels;
    use crate::domain::services::VideoOwnerFinder;
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::infrastructure::repositories::plex_library_repository::FakePlexLibraryRepository;
    use crate::infrastructure::repositories::sqlite_category_playlist_repository::SqliteCategoryPlaylistRepository;
    use crate::infrastructure::repositories::sqlite_category_repository::SqliteCategoryRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::SqliteChannelRepository;
    use crate::infrastructure::repositories::sqlite_channel_video_repository::SqliteChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_episode_labels_repository::{
        EpisodeLabelsRepository, SqliteEpisodeLabelsRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_categories_repository::SqliteVideoCategoriesRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Duration, Utc};
    use std::sync::Arc;

    #[test]
    fn it_should_mark_the_video_and_request_a_sync_in_a_minute() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let task_repository = task_repository(&db);
        let subscriber = SyncPlexLabelsOnVideoDownloaded::new(syncer(
            &db,
            episode_labels_repository.clone(),
            task_repository.clone(),
        ));

        let result = handle(&subscriber, "rec1");

        assert_eq!(result, Ok(()));
        assert_eq!(
            episode_labels_repository
                .find_many(&[record("rec1")])
                .unwrap(),
            vec![EpisodeLabels::pending(record("rec1"))]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_sync(1, now() + Duration::seconds(60))]
        );
    }

    /// The syncer over the test database, with a Plex server it never
    /// needs to reach.
    fn syncer(
        db: &TestDatabase,
        episode_labels_repository: Arc<dyn EpisodeLabelsRepository>,
        task_repository: Arc<dyn TaskRepository>,
    ) -> PlexLabelSyncer {
        PlexLabelSyncer::new(
            Arc::new(FakePlexLibraryRepository::failing()),
            episode_labels_repository,
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteCategoryRepository::new(db.database())),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            task_repository,
            Arc::new(FixedClock(now())),
            "/videos",
            "/plex/videos",
        )
    }

    fn task_repository(db: &TestDatabase) -> Arc<SqliteTaskRepository> {
        Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(now())),
        ))
    }

    /// The label sync pending as task `id`, due at `run_at`.
    fn pending_sync(id: i64, run_at: DateTime<Utc>) -> ScheduledTask {
        ScheduledTask {
            id,
            task_type: "sync_plex_labels".to_string(),
            payload: "{}".to_string(),
            status: TaskStatus::Pending,
            retries: 0,
            run_at,
            created_at: now(),
            updated_at: now(),
            last_error: None,
        }
    }

    fn record(id: &str) -> VideoRecordId {
        VideoRecordId::new(id).unwrap()
    }

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn handle(subscriber: &SyncPlexLabelsOnVideoDownloaded, video_id: &str) -> Result<(), String> {
        subscriber
            .handle(
                &serde_json::json!({
                    "video_id": video_id,
                    "output_dir": "/videos/creators/somechannel",
                    "folder": "Season 2023",
                })
                .to_string(),
            )
            .map_err(|e| e.to_string())
    }
}
