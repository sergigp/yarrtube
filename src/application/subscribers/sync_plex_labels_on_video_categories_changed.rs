use crate::domain::services::plex_label_syncer::CATEGORY_CHANGE_DELAY;
use crate::domain::services::{PlexLabelSyncer, PlexLabelSyncerApi};
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct VideoCategoriesChangedPayload {
    video_ids: Vec<String>,
}

/// Reacts to `VideoCategoriesChanged` by marking the records' Plex labels
/// stale and requesting a label sync shortly. Registered only when the Plex
/// integration is configured.
pub struct SyncPlexLabelsOnVideoCategoriesChanged {
    plex_label_syncer: PlexLabelSyncer,
}

impl SyncPlexLabelsOnVideoCategoriesChanged {
    pub fn new(plex_label_syncer: PlexLabelSyncer) -> Self {
        Self { plex_label_syncer }
    }
}

impl EventSubscriber for SyncPlexLabelsOnVideoCategoriesChanged {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoCategoriesChangedPayload = serde_json::from_str(payload)?;
        let video_ids = payload
            .video_ids
            .into_iter()
            .map(VideoRecordId::new)
            .collect::<Result<Vec<_>, _>>()?;
        self.plex_label_syncer
            .mark(&video_ids, CATEGORY_CHANGE_DELAY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plex::{PlexEpisode, PlexLabels};
    use crate::domain::plex_label::EpisodeLabels;
    use crate::domain::services::VideoOwnerFinder;
    use crate::domain::task::{ScheduledTask, Task, TaskStatus};
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
    fn it_should_mark_the_videos_and_request_a_sync_shortly() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let task_repository = task_repository(&db);
        episode_labels_repository
            .save_many(&[synced("rec1")])
            .unwrap();
        let subscriber = SyncPlexLabelsOnVideoCategoriesChanged::new(syncer(
            &db,
            episode_labels_repository.clone(),
            task_repository.clone(),
        ));

        let result = handle(&subscriber, &["rec1", "rec2"]);

        assert_eq!(result, Ok(()));
        assert_eq!(
            episode_labels_repository
                .find_many(&[record("rec1"), record("rec2")])
                .unwrap(),
            vec![
                EpisodeLabels {
                    dirty: true,
                    generation: 1,
                    ..synced("rec1")
                },
                EpisodeLabels::pending(record("rec2")),
            ]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_sync(1, now() + Duration::seconds(10))]
        );
    }

    #[test]
    fn it_should_bring_forward_a_pending_sync() {
        let db = TestDatabase::new();
        let task_repository = task_repository(&db);
        task_repository
            .schedule(&Task::SyncPlexLabels, now() + Duration::minutes(15))
            .unwrap();
        let subscriber = SyncPlexLabelsOnVideoCategoriesChanged::new(syncer(
            &db,
            Arc::new(SqliteEpisodeLabelsRepository::new(db.database())),
            task_repository.clone(),
        ));

        let result = handle(&subscriber, &["rec1"]);

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_sync(1, now() + Duration::seconds(10))]
        );
    }

    #[test]
    fn it_should_collapse_category_changes_into_one_sync() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let task_repository = task_repository(&db);
        let subscriber = SyncPlexLabelsOnVideoCategoriesChanged::new(syncer(
            &db,
            episode_labels_repository.clone(),
            task_repository.clone(),
        ));

        let results = [
            handle(&subscriber, &["rec1"]),
            handle(&subscriber, &["rec2"]),
        ];

        assert_eq!(results, [Ok(()), Ok(())]);
        assert_eq!(
            episode_labels_repository
                .find_many(&[record("rec1"), record("rec2")])
                .unwrap(),
            vec![
                EpisodeLabels::pending(record("rec1")),
                EpisodeLabels::pending(record("rec2")),
            ]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_sync(1, now() + Duration::seconds(10))]
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

    fn synced(id: &str) -> EpisodeLabels {
        EpisodeLabels::pending(record(id)).synced(
            &PlexEpisode {
                rating_key: "101".to_string(),
                section_id: "19".to_string(),
                file: "/plex/videos/a.mp4".to_string(),
            },
            PlexLabels::new(vec!["Chess".to_string()]),
            now(),
        )
    }

    fn record(id: &str) -> VideoRecordId {
        VideoRecordId::new(id).unwrap()
    }

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn handle(
        subscriber: &SyncPlexLabelsOnVideoCategoriesChanged,
        video_ids: &[&str],
    ) -> Result<(), String> {
        subscriber
            .handle(&serde_json::json!({ "video_ids": video_ids }).to_string())
            .map_err(|e| e.to_string())
    }
}
