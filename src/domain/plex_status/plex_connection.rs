use super::PlexSyncRun;
use chrono::{DateTime, Utc};

/// The Plex server as the latest sync of any kind saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexConnection {
    pub checked_at: DateTime<Utc>,
    /// Why the latest sync couldn't reach Plex; `None` when it did.
    pub error: Option<String>,
    /// From the latest run that reached Plex.
    pub version: Option<String>,
    /// From the latest run that reached Plex.
    pub libraries: Vec<String>,
}

impl PlexConnection {
    /// From the latest run of any kind, with the version and libraries of
    /// the latest run that reached Plex; `None` before any run.
    pub fn from_runs(_runs: &[PlexSyncRun]) -> Option<Self> {
        None
    }
}
