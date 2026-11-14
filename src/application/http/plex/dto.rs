use crate::domain::plex_status::{PlexConnection, PlexStatus, PlexSyncOutcome, PlexSyncStatus};
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct PlexStatusResponse {
    pub configured: bool,
    pub connection: Option<PlexConnectionResponse>,
    pub syncs: Vec<PlexSyncResponse>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PlexConnectionResponse {
    pub checked_at: DateTime<Utc>,
    pub reachable: bool,
    pub error: Option<String>,
    pub version: Option<String>,
    pub libraries: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PlexSyncResponse {
    pub kind: String,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub running: bool,
}

impl From<PlexStatus> for PlexStatusResponse {
    fn from(status: PlexStatus) -> Self {
        Self {
            configured: status.configured,
            connection: status.connection.map(PlexConnectionResponse::from),
            syncs: status
                .syncs
                .into_iter()
                .map(PlexSyncResponse::from)
                .collect(),
        }
    }
}

impl From<PlexConnection> for PlexConnectionResponse {
    fn from(connection: PlexConnection) -> Self {
        Self {
            checked_at: connection.checked_at,
            reachable: connection.error.is_none(),
            error: connection.error,
            version: connection.version,
            libraries: connection.libraries,
        }
    }
}

impl From<PlexSyncStatus> for PlexSyncResponse {
    fn from(sync: PlexSyncStatus) -> Self {
        Self {
            kind: sync.kind.as_str().to_string(),
            last_run_at: sync.last_run.as_ref().map(|run| run.finished_at),
            last_error: sync.last_run.and_then(|run| match run.outcome {
                PlexSyncOutcome::Failed { error } => Some(error),
                PlexSyncOutcome::Reached { .. } => None,
            }),
            next_run_at: sync.next_run_at,
            running: sync.running,
        }
    }
}
