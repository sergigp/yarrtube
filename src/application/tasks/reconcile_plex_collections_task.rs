use crate::domain::services::{PlexCollectionReconciler, PlexCollectionReconcilerApi};
use crate::domain::task::Task;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use crate::infrastructure::shared::error_report;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;
use tracing::{error, info};

/// Recurring global Plex collections convergence pass, reachable only via
/// the task queue (seeded once at daemon startup by
/// `schedule_reconcile_plex_collections_if_absent`, only when the Plex
/// integration is enabled). Like `UpdateYtdlpTask`, it always reschedules
/// its next occurrence and returns `Ok` whether or not this pass succeeded —
/// a failed pass is logged and retried by the next one, never by the task
/// queue's retry/dead-letter machinery, whose retrying attempts would each
/// seed yet another recurring chain.
pub struct ReconcilePlexCollectionsTask {
    reconciler: PlexCollectionReconciler,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    interval_seconds: i64,
}

impl ReconcilePlexCollectionsTask {
    pub fn new(
        reconciler: PlexCollectionReconciler,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
        interval_seconds: i64,
    ) -> Self {
        Self {
            reconciler,
            task_repository,
            clock,
            interval_seconds,
        }
    }
}

impl TaskHandler for ReconcilePlexCollectionsTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        Task::decode_reconcile_plex_collections_payload(payload)?;
        match self.reconciler.reconcile_all() {
            Ok(()) => info!("Plex collections reconcile pass succeeded"),
            Err(e) => error!(
                error = %error_report::cause_chain(&e),
                "Plex collections reconcile pass failed, retrying on the next scheduled pass"
            ),
        }

        let next_run_at = self.clock.now() + chrono::Duration::seconds(self.interval_seconds);
        self.task_repository
            .schedule(&Task::ReconcilePlexCollections, next_run_at)?;
        info!(next_run_at = %next_run_at, "scheduled next Plex collections reconcile");

        Ok(())
    }
}

/// Seeds the recurring `reconcile_plex_collections` task chain to run now,
/// unless a non-terminal (`pending` or `running`) one is already scheduled —
/// a restarted daemon must not stack up additional chains alongside one that
/// already self-perpetuates forever.
pub fn schedule_reconcile_plex_collections_if_absent(
    task_repository: &Arc<dyn TaskRepository>,
    clock: &Arc<dyn Clock>,
) -> anyhow::Result<()> {
    let existing = task_repository
        .list_non_completed()?
        .into_iter()
        .find(|task| task.task_type == Task::ReconcilePlexCollections.task_type());

    match existing {
        Some(task) => info!(
            task_id = task.id,
            "recurring Plex collections reconcile task already scheduled, skipping seed"
        ),
        None => {
            let run_at = clock.now();
            task_repository.schedule(&Task::ReconcilePlexCollections, run_at)?;
            info!(run_at = %run_at, "scheduled recurring Plex collections reconcile task");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::playlist::{Playlist, PlaylistId, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::plex::{PlexItem, PlexMatchCandidate};
    use crate::domain::services::PlexCollectionReconciler;
    use crate::domain::shared::Quality;
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::domain::video::{Video, VideoId};
    use crate::infrastructure::repositories::plex_collection_repository::{
        FakePlexCollection, FakePlexCollectionRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, SqliteChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, SqliteChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        PlaylistRepository, SqlitePlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::{
        PlaylistVideoRepository, SqlitePlaylistVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    const INTERVAL_SECONDS: i64 = 900;

    #[test]
    fn it_should_skip_creating_a_collection_if_no_video_is_scanned_yet() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items("1", vec![]));
        playlist_repository
            .insert(&playlist("PL1", "Lofi beats"))
            .unwrap();
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt1",
            0,
        );
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                playlist_repository.clone(),
                Arc::new(SqliteChannelRepository::new(db.database())),
                playlist_video_repository.clone(),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                video_repository.clone(),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(plex_repository.collections(), vec![]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                fixed_timestamp() + chrono::Duration::seconds(INTERVAL_SECONDS),
            )]
        );
    }

    #[test]
    fn it_should_rematch_an_unidentified_item_and_collect_it_next_pass() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(
            FakePlexCollectionRepository::with_items("1", vec![unidentified_plex_item("101")])
                .with_match_candidates("101", vec![nfo_candidate("yt1")]),
        );
        playlist_repository
            .insert(&playlist("PL1", "Lofi beats"))
            .unwrap();
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt1",
            0,
        );
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                playlist_repository.clone(),
                Arc::new(SqliteChannelRepository::new(db.database())),
                playlist_video_repository.clone(),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                video_repository.clone(),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let first = run(&task, "{}");
        let collections_after_first = plex_repository.collections();
        let second = run(&task, "{}");

        assert_eq!((first, second), (Ok(()), Ok(())));
        assert_eq!(collections_after_first, vec![]);
        assert_eq!(
            plex_repository.collections(),
            vec![FakePlexCollection {
                rating_key: "collection:Lofi beats".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec!["101".to_string()],
            }]
        );
        assert_eq!(
            plex_repository.mutations(),
            vec![
                "match:101:tv.plex.agents.nfo.movie://movie/youtube_yt1".to_string(),
                "create:1:Lofi beats".to_string(),
            ]
        );
    }

    #[test]
    fn it_should_skip_an_unidentified_item_without_a_youtube_candidate() {
        let db = TestDatabase::new();
        let plex_repository = Arc::new(
            FakePlexCollectionRepository::with_items(
                "1",
                vec![unidentified_plex_item("101"), unidentified_plex_item("102")],
            )
            .with_match_candidates(
                "101",
                vec![PlexMatchCandidate {
                    guid: "plex://movie/5d776830880197001ec90f22".to_string(),
                    name: "Some movie".to_string(),
                }],
            ),
        );
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqliteVideoRepository::new(db.database())),
                plex_repository.clone(),
            ),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(plex_repository.mutations(), Vec::<String>::new());
        assert_eq!(
            plex_repository.items(),
            vec![unidentified_plex_item("101"), unidentified_plex_item("102")]
        );
    }

    #[test]
    fn it_should_continue_the_pass_if_a_rematch_fails() {
        let db = TestDatabase::new();
        let plex_repository = Arc::new(
            FakePlexCollectionRepository::with_items(
                "1",
                vec![unidentified_plex_item("101"), unidentified_plex_item("102")],
            )
            .with_match_candidates("101", vec![nfo_candidate("yt1")])
            .with_match_candidates("102", vec![nfo_candidate("yt2")])
            .failing_match_for("101"),
        );
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqliteVideoRepository::new(db.database())),
                plex_repository.clone(),
            ),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.mutations(),
            vec!["match:102:tv.plex.agents.nfo.movie://movie/youtube_yt2".to_string()]
        );
    }

    #[test]
    fn it_should_not_look_up_matches_if_every_item_is_identified() {
        let db = TestDatabase::new();
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items(
            "1",
            vec![plex_item("101", "yt1")],
        ));
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqliteVideoRepository::new(db.database())),
                plex_repository.clone(),
            ),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(plex_repository.match_lookups(), Vec::<String>::new());
    }

    #[test]
    fn it_should_ignore_videos_that_are_not_downloaded() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items(
            "1",
            vec![plex_item("101", "yt1"), plex_item("102", "yt2")],
        ));
        playlist_repository
            .insert(&playlist("PL1", "Lofi beats"))
            .unwrap();
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt1",
            0,
        );
        let pending = Video::create(VideoId::new("yt2").unwrap(), "My Video", fixed_timestamp());
        video_repository.save(&pending).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                pending.id,
                1,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                playlist_repository.clone(),
                Arc::new(SqliteChannelRepository::new(db.database())),
                playlist_video_repository.clone(),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                video_repository.clone(),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.collections(),
            vec![FakePlexCollection {
                rating_key: "collection:Lofi beats".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec!["101".to_string()],
            }]
        );
    }

    #[test]
    fn it_should_add_newly_scanned_videos_to_an_existing_collection() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items_and_collections(
            "1",
            vec![plex_item("101", "yt1"), plex_item("102", "yt2")],
            vec![FakePlexCollection {
                rating_key: "c1".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec!["101".to_string()],
            }],
        ));
        playlist_repository
            .insert(&playlist("PL1", "Lofi beats"))
            .unwrap();
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt1",
            0,
        );
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt2",
            1,
        );
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                playlist_repository.clone(),
                Arc::new(SqliteChannelRepository::new(db.database())),
                playlist_video_repository.clone(),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                video_repository.clone(),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.collections(),
            vec![FakePlexCollection {
                rating_key: "c1".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec!["101".to_string(), "102".to_string()],
            }]
        );
    }

    #[test]
    fn it_should_recreate_an_empty_collection_with_its_scanned_videos() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items_and_collections(
            "1",
            vec![plex_item("101", "yt1"), plex_item("102", "yt2")],
            vec![FakePlexCollection {
                rating_key: "c1".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec![],
            }],
        ));
        playlist_repository
            .insert(&playlist("PL1", "Lofi beats"))
            .unwrap();
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt1",
            0,
        );
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt2",
            1,
        );
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                playlist_repository.clone(),
                Arc::new(SqliteChannelRepository::new(db.database())),
                playlist_video_repository.clone(),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                video_repository.clone(),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.mutations(),
            vec!["delete:c1".to_string(), "create:1:Lofi beats".to_string()]
        );
        assert_eq!(
            plex_repository.collections(),
            vec![FakePlexCollection {
                rating_key: "collection:Lofi beats".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec!["101".to_string(), "102".to_string()],
            }]
        );
    }

    #[test]
    fn it_should_leave_an_empty_collection_alone_if_no_video_is_scanned() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items_and_collections(
            "1",
            vec![],
            vec![FakePlexCollection {
                rating_key: "c1".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec![],
            }],
        ));
        playlist_repository
            .insert(&playlist("PL1", "Lofi beats"))
            .unwrap();
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt1",
            0,
        );
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                playlist_repository.clone(),
                Arc::new(SqliteChannelRepository::new(db.database())),
                playlist_video_repository.clone(),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                video_repository.clone(),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(plex_repository.mutations(), Vec::<String>::new());
        assert_eq!(
            plex_repository.collections(),
            vec![FakePlexCollection {
                rating_key: "c1".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec![],
            }]
        );
    }

    #[test]
    fn it_should_remove_videos_no_longer_tracked_from_the_collection() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items_and_collections(
            "1",
            vec![plex_item("101", "yt1"), plex_item("102", "yt2")],
            vec![FakePlexCollection {
                rating_key: "c1".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec!["101".to_string(), "102".to_string()],
            }],
        ));
        playlist_repository
            .insert(&playlist("PL1", "Lofi beats"))
            .unwrap();
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt1",
            0,
        );
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                playlist_repository.clone(),
                Arc::new(SqliteChannelRepository::new(db.database())),
                playlist_video_repository.clone(),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                video_repository.clone(),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.collections(),
            vec![FakePlexCollection {
                rating_key: "c1".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec!["101".to_string()],
            }]
        );
    }

    #[test]
    fn it_should_do_nothing_if_already_in_sync() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items_and_collections(
            "1",
            vec![plex_item("101", "yt1"), plex_item("102", "yt2")],
            vec![FakePlexCollection {
                rating_key: "c1".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec!["101".to_string(), "102".to_string()],
            }],
        ));
        playlist_repository
            .insert(&playlist("PL1", "Lofi beats"))
            .unwrap();
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt1",
            0,
        );
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt2",
            1,
        );
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                playlist_repository.clone(),
                Arc::new(SqliteChannelRepository::new(db.database())),
                playlist_video_repository.clone(),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                video_repository.clone(),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(plex_repository.mutations(), Vec::<String>::new());
        assert_eq!(
            plex_repository.collections(),
            vec![FakePlexCollection {
                rating_key: "c1".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec!["101".to_string(), "102".to_string()],
            }]
        );
    }

    #[test]
    fn it_should_continue_reconciling_remaining_collections_if_one_fails() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(
            FakePlexCollectionRepository::with_items(
                "1",
                vec![plex_item("101", "yt1"), plex_item("102", "yt2")],
            )
            .failing_create_for("Alpha"),
        );
        playlist_repository
            .insert(&playlist("PL1", "Alpha"))
            .unwrap();
        playlist_repository
            .insert(&playlist("PL2", "Beta"))
            .unwrap();
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt1",
            0,
        );
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL2",
            "yt2",
            0,
        );
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                playlist_repository.clone(),
                Arc::new(SqliteChannelRepository::new(db.database())),
                playlist_video_repository.clone(),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                video_repository.clone(),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.collections(),
            vec![FakePlexCollection {
                rating_key: "collection:Beta".to_string(),
                title: "Beta".to_string(),
                member_rating_keys: vec!["102".to_string()],
            }]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                fixed_timestamp() + chrono::Duration::seconds(INTERVAL_SECONDS),
            )]
        );
    }

    #[test]
    fn it_should_reschedule_the_next_reconcile_if_the_pass_fails() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(FakePlexCollectionRepository::failing());
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqliteVideoRepository::new(db.database())),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                fixed_timestamp() + chrono::Duration::seconds(INTERVAL_SECONDS),
            )]
        );
    }

    #[test]
    fn it_should_create_collections_in_their_own_sections() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(
            FakePlexCollectionRepository::with_items("2", vec![plex_item("101", "yt1")])
                .and_section("5", vec![plex_item("201", "yt2")], vec![]),
        );
        playlist_repository
            .insert(&playlist("PL1", "Lofi beats"))
            .unwrap();
        channel_repository
            .insert(&channel("@somechannel", "Some Channel"))
            .unwrap();
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt1",
            0,
        );
        seed_downloaded_channel_video(
            &channel_video_repository,
            &video_repository,
            "@somechannel",
            "yt2",
            0,
        );
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["2".to_string()],
                vec!["5".to_string()],
                playlist_repository.clone(),
                channel_repository.clone(),
                playlist_video_repository.clone(),
                channel_video_repository.clone(),
                video_repository.clone(),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.collections_in("2"),
            vec![FakePlexCollection {
                rating_key: "collection:Lofi beats".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec!["101".to_string()],
            }]
        );
        assert_eq!(
            plex_repository.collections_in("5"),
            vec![FakePlexCollection {
                rating_key: "collection:Some Channel".to_string(),
                title: "Some Channel".to_string(),
                member_rating_keys: vec!["201".to_string()],
            }]
        );
    }

    #[test]
    fn it_should_not_create_a_channel_collection_in_a_playlist_section() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items(
            "1",
            vec![plex_item("101", "yt1")],
        ));
        playlist_repository
            .insert(&playlist("PL1", "Lofi beats"))
            .unwrap();
        channel_repository
            .insert(&channel("@somechannel", "Some Channel"))
            .unwrap();
        // The same downloaded video sits in both the tracked playlist and the
        // tracked channel, and is scanned into this (playlist) section.
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp())
            .mark_downloaded(Quality::High, "My Video.mp4", None, None, fixed_timestamp());
        video_repository.save(&video).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                ChannelHandle::new("@somechannel").unwrap(),
                video.id,
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                playlist_repository.clone(),
                channel_repository.clone(),
                playlist_video_repository.clone(),
                channel_video_repository.clone(),
                video_repository.clone(),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.collections(),
            vec![FakePlexCollection {
                rating_key: "collection:Lofi beats".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec!["101".to_string()],
            }]
        );
    }

    #[test]
    fn it_should_create_a_collection_for_a_channel_with_scanned_videos() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items(
            "1",
            vec![plex_item("101", "yt1"), plex_item("102", "yt2")],
        ));
        channel_repository
            .insert(&channel("@somechannel", "Some Channel"))
            .unwrap();
        seed_downloaded_channel_video(
            &channel_video_repository,
            &video_repository,
            "@somechannel",
            "yt1",
            0,
        );
        seed_downloaded_channel_video(
            &channel_video_repository,
            &video_repository,
            "@somechannel",
            "yt2",
            1,
        );
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec![],
                vec!["1".to_string()],
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                channel_repository.clone(),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_video_repository.clone(),
                video_repository.clone(),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.collections(),
            vec![FakePlexCollection {
                rating_key: "collection:Some Channel".to_string(),
                title: "Some Channel".to_string(),
                member_rating_keys: vec!["101".to_string(), "102".to_string()],
            }]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                fixed_timestamp() + chrono::Duration::seconds(INTERVAL_SECONDS),
            )]
        );
    }

    #[test]
    fn it_should_create_a_collection_for_a_playlist_with_scanned_videos() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let plex_repository = Arc::new(FakePlexCollectionRepository::with_items(
            "1",
            vec![plex_item("101", "yt1"), plex_item("102", "yt2")],
        ));
        playlist_repository
            .insert(&playlist("PL1", "Lofi beats"))
            .unwrap();
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt1",
            0,
        );
        seed_downloaded_playlist_video(
            &playlist_video_repository,
            &video_repository,
            "PL1",
            "yt2",
            1,
        );
        let task = ReconcilePlexCollectionsTask::new(
            PlexCollectionReconciler::new(
                vec!["1".to_string()],
                vec![],
                playlist_repository.clone(),
                Arc::new(SqliteChannelRepository::new(db.database())),
                playlist_video_repository.clone(),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                video_repository.clone(),
                plex_repository.clone(),
            ),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            INTERVAL_SECONDS,
        );

        let result = run(&task, "{}");

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.collections(),
            vec![FakePlexCollection {
                rating_key: "collection:Lofi beats".to_string(),
                title: "Lofi beats".to_string(),
                member_rating_keys: vec!["101".to_string(), "102".to_string()],
            }]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                fixed_timestamp() + chrono::Duration::seconds(INTERVAL_SECONDS),
            )]
        );
    }

    fn unidentified_plex_item(rating_key: &str) -> PlexItem {
        PlexItem {
            rating_key: rating_key.to_string(),
            youtube_video_id: None,
        }
    }

    /// The candidate Plex's NFO agent offers for an item whose `movie.nfo`
    /// holds `youtube_video_id`.
    fn nfo_candidate(youtube_video_id: &str) -> PlexMatchCandidate {
        PlexMatchCandidate {
            guid: format!("tv.plex.agents.nfo.movie://movie/youtube_{youtube_video_id}"),
            name: "Some video".to_string(),
        }
    }

    fn plex_item(rating_key: &str, youtube_video_id: &str) -> PlexItem {
        PlexItem {
            rating_key: rating_key.to_string(),
            youtube_video_id: Some(youtube_video_id.to_string()),
        }
    }

    fn channel(handle: &str, name: &str) -> Channel {
        Channel::create(
            ChannelHandle::new(handle).unwrap(),
            name,
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    /// Saves a downloaded video and its channel membership at `position`.
    fn seed_downloaded_channel_video(
        channel_video_repository: &Arc<SqliteChannelVideoRepository>,
        video_repository: &Arc<SqliteVideoRepository>,
        channel_id: &str,
        youtube_id: &str,
        position: i64,
    ) {
        let video = Video::create(
            VideoId::new(youtube_id).unwrap(),
            "My Video",
            fixed_timestamp(),
        )
        .mark_downloaded(Quality::High, "My Video.mp4", None, None, fixed_timestamp());
        video_repository.save(&video).unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                ChannelHandle::new(channel_id).unwrap(),
                video.id,
                position,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    fn playlist(id: &str, name: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new(name).unwrap(),
            PlaylistPath::new("music/chill").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            false,
            fixed_timestamp(),
        )
    }

    /// Saves a downloaded video and its playlist membership at `position`.
    fn seed_downloaded_playlist_video(
        playlist_video_repository: &Arc<SqlitePlaylistVideoRepository>,
        video_repository: &Arc<SqliteVideoRepository>,
        playlist_id: &str,
        youtube_id: &str,
        position: i64,
    ) {
        let video = Video::create(
            VideoId::new(youtube_id).unwrap(),
            "My Video",
            fixed_timestamp(),
        )
        .mark_downloaded(Quality::High, "My Video.mp4", None, None, fixed_timestamp());
        video_repository.save(&video).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new(playlist_id).unwrap(),
                video.id,
                position,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    fn pending_task(id: i64, run_at: DateTime<Utc>) -> ScheduledTask {
        ScheduledTask {
            id,
            task_type: Task::ReconcilePlexCollections.task_type().to_string(),
            payload: Task::ReconcilePlexCollections.payload().to_string(),
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

    fn run(task: &ReconcilePlexCollectionsTask, payload: &str) -> Result<(), String> {
        task.handle(payload, false).map_err(|e| e.to_string())
    }
}
