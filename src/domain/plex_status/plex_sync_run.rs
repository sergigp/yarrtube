use super::{PlexSyncKind, PlexSyncOutcome};
use chrono::{DateTime, Utc};

/// The latest finished run of a Plex sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexSyncRun {
    pub kind: PlexSyncKind,
    pub finished_at: DateTime<Utc>,
    pub outcome: PlexSyncOutcome,
}
