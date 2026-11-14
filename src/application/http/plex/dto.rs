use crate::domain::plex_status::PlexStatus;
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct PlexStatusResponse {
    pub configured: bool,
    pub connection: Option<PlexConnectionResponse>,
    pub syncs: Vec<PlexSyncResponse>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PlexConnectionResponse {
    pub checked_at: String,
    pub reachable: bool,
    pub error: Option<String>,
    pub version: Option<String>,
    pub libraries: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PlexSyncResponse {
    pub kind: String,
    pub last_run_at: Option<String>,
    pub last_error: Option<String>,
    pub next_run_at: Option<String>,
    pub running: bool,
}

impl From<PlexStatus> for PlexStatusResponse {
    fn from(_status: PlexStatus) -> Self {
        Self {
            configured: false,
            connection: None,
            syncs: Vec::new(),
        }
    }
}
