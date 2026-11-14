use super::{PlexSyncKind, PlexSyncRun};
use chrono::{DateTime, Utc};

/// Where one Plex sync stands: its latest run, its next and whether it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexSyncStatus {
    pub kind: PlexSyncKind,
    pub last_run: Option<PlexSyncRun>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub running: bool,
}
