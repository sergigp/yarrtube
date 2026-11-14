use super::*;
use crate::domain::task::Task;
use crate::infrastructure::repositories::sqlite_plex_sync_run_repository::SqlitePlexSyncRunRepository;
use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::sqlite_connection::TestDatabase;
use crate::infrastructure::shared::system_clock::FixedClock;
use chrono::{DateTime, Duration, Utc};
use dto::{PlexStatusResponse, PlexSyncResponse};
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

fn now() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
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

async fn status(plex_status_searcher: PlexStatusSearcher) -> Result<PlexStatusResponse, ApiError> {
    get_plex_status(State(plex_status_searcher))
        .await
        .map(|Json(response)| response)
}
