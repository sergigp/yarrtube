use super::{PlexConnection, PlexSyncStatus};

/// How the Plex integration is doing, from what its syncs recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexStatus {
    pub configured: bool,
    pub connection: Option<PlexConnection>,
    pub syncs: Vec<PlexSyncStatus>,
}
