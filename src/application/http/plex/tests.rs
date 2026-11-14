use super::*;
use crate::domain::plex_status::{PlexSyncKind, PlexSyncOutcome, PlexSyncRun};
use crate::domain::services::{
    CategoryVideoCollector, PlexEpisodeFinder, PlexLabelSyncer, PlexWatchSyncer, VideoOwnerFinder,
    VideoWatchStateUpdater,
};
use crate::domain::task::{ScheduledTask, Task, TaskStatus};
use crate::infrastructure::repositories::plex_library_repository::FakePlexLibraryRepository;
use crate::infrastructure::repositories::sqlite_category_playlist_repository::SqliteCategoryPlaylistRepository;
use crate::infrastructure::repositories::sqlite_category_repository::SqliteCategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::SqliteChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::SqliteChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_episode_labels_repository::SqliteEpisodeLabelsRepository;
use crate::infrastructure::repositories::sqlite_episode_watch_repository::SqliteEpisodeWatchRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_plex_sync_run_repository::{
    PlexSyncRunRepository, SqlitePlexSyncRunRepository,
};
use crate::infrastructure::repositories::sqlite_preferences_repository::SqlitePreferencesRepository;
use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_categories_repository::SqliteVideoCategoriesRepository;
use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
use crate::infrastructure::shared::sqlite_connection::TestDatabase;
use crate::infrastructure::shared::system_clock::FixedClock;
use chrono::{DateTime, Duration, Utc};
use dto::{PlexConnectionResponse, PlexStatusResponse, PlexSyncResponse};
use std::sync::Arc;

#[tokio::test]
async fn it_should_report_plex_not_configured() {
    let db = TestDatabase::new();
    let plex_status_searcher = PlexStatusSearcher::new(
        false,
        Arc::new(SqlitePlexSyncRunRepository::new(db.database())),
        Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(now())),
        )),
    );

    let response = status(plex_status_searcher).await;

    assert_eq!(
        response,
        Ok(PlexStatusResponse {
            configured: false,
            connection: None,
            syncs: vec![],
        })
    );
}

#[tokio::test]
async fn it_should_report_each_sync_with_its_next_run_before_any_run() {
    let db = TestDatabase::new();
    let task_repository = Arc::new(SqliteTaskRepository::new(
        db.database(),
        Arc::new(FixedClock(now())),
    ));
    task_repository
        .schedule(&Task::SyncPlexWatchState, now() + Duration::minutes(5))
        .unwrap();
    task_repository
        .schedule(&Task::SyncPlexLabels, now() + Duration::minutes(15))
        .unwrap();
    let plex_status_searcher = PlexStatusSearcher::new(
        true,
        Arc::new(SqlitePlexSyncRunRepository::new(db.database())),
        task_repository,
    );

    let response = status(plex_status_searcher).await;

    assert_eq!(
        response,
        Ok(PlexStatusResponse {
            configured: true,
            connection: None,
            syncs: vec![
                PlexSyncResponse {
                    next_run_at: Some(now() + Duration::minutes(5)),
                    ..never_run_sync("watch_state")
                },
                PlexSyncResponse {
                    next_run_at: Some(now() + Duration::minutes(15)),
                    ..never_run_sync("labels")
                },
            ],
        })
    );
}

#[tokio::test]
async fn it_should_report_the_connection_from_the_latest_run() {
    let db = TestDatabase::new();
    let plex_sync_run_repository = Arc::new(SqlitePlexSyncRunRepository::new(db.database()));
    let two_minutes_ago = now() - Duration::minutes(2);
    plex_sync_run_repository
        .save(&PlexSyncRun {
            kind: PlexSyncKind::WatchState,
            finished_at: two_minutes_ago,
            outcome: reached(),
        })
        .unwrap();
    let plex_status_searcher =
        PlexStatusSearcher::new(true, plex_sync_run_repository, task_repository(&db));

    let response = status(plex_status_searcher).await;

    assert_eq!(
        response,
        Ok(PlexStatusResponse {
            configured: true,
            connection: Some(PlexConnectionResponse {
                checked_at: two_minutes_ago,
                reachable: true,
                error: None,
                version: Some("1.43.5.11079-b3b13e506".to_string()),
                libraries: vec!["Subscriptions".to_string(), "Sergi's Playlists".to_string()],
            }),
            syncs: vec![
                PlexSyncResponse {
                    last_run_at: Some(two_minutes_ago),
                    ..never_run_sync("watch_state")
                },
                never_run_sync("labels"),
            ],
        })
    );
}

#[tokio::test]
async fn it_should_keep_the_version_and_libraries_last_seen_when_the_latest_run_failed() {
    let db = TestDatabase::new();
    let plex_sync_run_repository = Arc::new(SqlitePlexSyncRunRepository::new(db.database()));
    let ten_minutes_ago = now() - Duration::minutes(10);
    let five_minutes_ago = now() - Duration::minutes(5);
    plex_sync_run_repository
        .save(&PlexSyncRun {
            kind: PlexSyncKind::WatchState,
            finished_at: ten_minutes_ago,
            outcome: reached(),
        })
        .unwrap();
    plex_sync_run_repository
        .save(&PlexSyncRun {
            kind: PlexSyncKind::Labels,
            finished_at: five_minutes_ago,
            outcome: PlexSyncOutcome::Failed {
                error: "timed out".to_string(),
            },
        })
        .unwrap();
    let plex_status_searcher =
        PlexStatusSearcher::new(true, plex_sync_run_repository, task_repository(&db));

    let response = status(plex_status_searcher).await;

    assert_eq!(
        response,
        Ok(PlexStatusResponse {
            configured: true,
            connection: Some(PlexConnectionResponse {
                checked_at: five_minutes_ago,
                reachable: false,
                error: Some("timed out".to_string()),
                version: Some("1.43.5.11079-b3b13e506".to_string()),
                libraries: vec!["Subscriptions".to_string(), "Sergi's Playlists".to_string()],
            }),
            syncs: vec![
                PlexSyncResponse {
                    last_run_at: Some(ten_minutes_ago),
                    ..never_run_sync("watch_state")
                },
                PlexSyncResponse {
                    last_run_at: Some(five_minutes_ago),
                    last_error: Some("timed out".to_string()),
                    ..never_run_sync("labels")
                },
            ],
        })
    );
}

#[tokio::test]
async fn it_should_report_a_running_sync() {
    let db = TestDatabase::new();
    let task_repository = task_repository(&db);
    task_repository
        .schedule(&Task::SyncPlexLabels, now())
        .unwrap();
    task_repository.claim(1, now()).unwrap();
    let plex_status_searcher = PlexStatusSearcher::new(
        true,
        Arc::new(SqlitePlexSyncRunRepository::new(db.database())),
        task_repository,
    );

    let response = status(plex_status_searcher).await;

    assert_eq!(
        response,
        Ok(PlexStatusResponse {
            configured: true,
            connection: None,
            syncs: vec![
                never_run_sync("watch_state"),
                PlexSyncResponse {
                    running: true,
                    ..never_run_sync("labels")
                },
            ],
        })
    );
}

#[tokio::test]
async fn it_should_bring_the_pending_sync_forward_when_synced_now() {
    let db = TestDatabase::new();
    let task_repository = task_repository(&db);
    task_repository
        .schedule(&Task::SyncPlexWatchState, now() + Duration::minutes(4))
        .unwrap();
    let plex_sync_requester = PlexSyncRequester::new(
        Some(watch_syncer(&db, task_repository.clone())),
        Some(label_syncer(&db, task_repository.clone())),
        Arc::new(FixedClock(now())),
    );

    let response = sync_now(plex_sync_requester, "watch_state").await;

    assert_eq!(response, Ok(StatusCode::NO_CONTENT));
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![pending_sync(1, Task::SyncPlexWatchState)]
    );
}

#[tokio::test]
async fn it_should_schedule_a_sync_now_when_none_is_pending() {
    let db = TestDatabase::new();
    let task_repository = task_repository(&db);
    let plex_sync_requester = PlexSyncRequester::new(
        Some(watch_syncer(&db, task_repository.clone())),
        Some(label_syncer(&db, task_repository.clone())),
        Arc::new(FixedClock(now())),
    );

    let response = sync_now(plex_sync_requester, "labels").await;

    assert_eq!(response, Ok(StatusCode::NO_CONTENT));
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![pending_sync(1, Task::SyncPlexLabels)]
    );
}

#[tokio::test]
async fn it_should_leave_one_pending_sync_when_synced_now_twice() {
    let db = TestDatabase::new();
    let task_repository = task_repository(&db);
    let plex_sync_requester = PlexSyncRequester::new(
        Some(watch_syncer(&db, task_repository.clone())),
        Some(label_syncer(&db, task_repository.clone())),
        Arc::new(FixedClock(now())),
    );
    sync_now(plex_sync_requester.clone(), "labels")
        .await
        .unwrap();

    let response = sync_now(plex_sync_requester, "labels").await;

    assert_eq!(response, Ok(StatusCode::NO_CONTENT));
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![pending_sync(1, Task::SyncPlexLabels)]
    );
}

#[tokio::test]
async fn it_should_respond_404_for_an_unknown_sync() {
    let db = TestDatabase::new();
    let task_repository = task_repository(&db);
    let plex_sync_requester = PlexSyncRequester::new(
        Some(watch_syncer(&db, task_repository.clone())),
        Some(label_syncer(&db, task_repository.clone())),
        Arc::new(FixedClock(now())),
    );

    let response = sync_now(plex_sync_requester, "scan").await;

    assert_eq!(
        response,
        Err(ApiError::new(StatusCode::NOT_FOUND, "No Plex sync scan"))
    );
    assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
}

#[tokio::test]
async fn it_should_respond_409_when_plex_isnt_configured() {
    let db = TestDatabase::new();
    let task_repository = task_repository(&db);
    let plex_sync_requester = PlexSyncRequester::new(None, None, Arc::new(FixedClock(now())));

    let response = sync_now(plex_sync_requester, "watch_state").await;

    assert_eq!(
        response,
        Err(ApiError::new(StatusCode::CONFLICT, "Plex isn't configured"))
    );
    assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
}

fn now() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
}

fn watch_syncer(db: &TestDatabase, task_repository: Arc<SqliteTaskRepository>) -> PlexWatchSyncer {
    let plex = Arc::new(FakePlexLibraryRepository::default());
    let clock = Arc::new(FixedClock(now()));
    PlexWatchSyncer::new(
        plex.clone(),
        plex_episode_finder(db, plex),
        Arc::new(SqliteEpisodeWatchRepository::new(db.database())),
        Arc::new(SqliteVideoRepository::new(db.database())),
        VideoWatchStateUpdater::new(
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            clock.clone(),
            Arc::new(SqliteCategoryRepository::new(db.database())),
            CategoryVideoCollector::new(
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteVideoRepository::new(db.database())),
                Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            ),
        ),
        task_repository,
        Arc::new(SqlitePlexSyncRunRepository::new(db.database())),
        clock,
    )
}

fn label_syncer(db: &TestDatabase, task_repository: Arc<SqliteTaskRepository>) -> PlexLabelSyncer {
    let plex = Arc::new(FakePlexLibraryRepository::default());
    PlexLabelSyncer::new(
        plex.clone(),
        Arc::new(SqliteEpisodeLabelsRepository::new(db.database())),
        Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
        Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
        Arc::new(SqliteCategoryRepository::new(db.database())),
        Arc::new(SqliteVideoRepository::new(db.database())),
        plex_episode_finder(db, plex),
        task_repository,
        Arc::new(SqlitePreferencesRepository::new(db.database())),
        Arc::new(SqlitePlexSyncRunRepository::new(db.database())),
        Arc::new(FixedClock(now())),
    )
}

fn plex_episode_finder(
    db: &TestDatabase,
    plex: Arc<FakePlexLibraryRepository>,
) -> PlexEpisodeFinder {
    PlexEpisodeFinder::new(
        plex,
        Arc::new(VideoOwnerFinder::new(
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            "/videos",
        )),
        "/videos",
        "/plex/videos",
    )
}

fn pending_sync(id: i64, task: Task) -> ScheduledTask {
    ScheduledTask {
        id,
        task,
        status: TaskStatus::Pending,
        retries: 0,
        run_at: now(),
        created_at: now(),
        updated_at: now(),
        last_error: None,
    }
}

fn task_repository(db: &TestDatabase) -> Arc<SqliteTaskRepository> {
    Arc::new(SqliteTaskRepository::new(
        db.database(),
        Arc::new(FixedClock(now())),
    ))
}

fn reached() -> PlexSyncOutcome {
    PlexSyncOutcome::Reached {
        version: "1.43.5.11079-b3b13e506".to_string(),
        libraries: vec!["Subscriptions".to_string(), "Sergi's Playlists".to_string()],
    }
}

fn never_run_sync(kind: &str) -> PlexSyncResponse {
    PlexSyncResponse {
        kind: kind.to_string(),
        last_run_at: None,
        last_error: None,
        next_run_at: None,
        running: false,
    }
}

async fn sync_now(
    plex_sync_requester: PlexSyncRequester,
    kind: &str,
) -> Result<StatusCode, ApiError> {
    request_plex_sync(State(plex_sync_requester), Path(kind.to_string())).await
}

async fn status(plex_status_searcher: PlexStatusSearcher) -> Result<PlexStatusResponse, ApiError> {
    get_plex_status(State(plex_status_searcher))
        .await
        .map(|Json(response)| response)
}
