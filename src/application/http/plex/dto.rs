use crate::domain::plex_status::{PlexStatus, PlexSyncStatus};
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
            connection: None,
            syncs: status
                .syncs
                .into_iter()
                .map(PlexSyncResponse::from)
                .collect(),
        }
    }
}

impl From<PlexSyncStatus> for PlexSyncResponse {
    fn from(sync: PlexSyncStatus) -> Self {
        Self {
            kind: sync.kind.as_str().to_string(),
            last_run_at: None,
            last_error: None,
            next_run_at: sync.next_run_at,
            running: sync.running,
        }
    }
}
