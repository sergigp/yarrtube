use super::*;
use crate::infrastructure::repositories::sqlite_plex_sync_run_repository::SqlitePlexSyncRunRepository;
use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
use crate::infrastructure::shared::sqlite_connection::TestDatabase;
use crate::infrastructure::shared::system_clock::FixedClock;
use chrono::{DateTime, Utc};
use dto::PlexStatusResponse;
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

fn now() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
}

async fn status(plex_status_searcher: PlexStatusSearcher) -> Result<PlexStatusResponse, ApiError> {
    get_plex_status(State(plex_status_searcher))
        .await
        .map(|Json(response)| response)
}
