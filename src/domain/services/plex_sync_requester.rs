use crate::domain::plex_status::{PlexSyncKind, RequestPlexSyncError};
use crate::domain::services::{PlexLabelSyncer, PlexWatchSyncer};
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Makes a Plex sync due now.
#[derive(Clone)]
pub struct PlexSyncRequester {
    #[allow(dead_code)]
    watch_syncer: Option<PlexWatchSyncer>,
    #[allow(dead_code)]
    label_syncer: Option<PlexLabelSyncer>,
    #[allow(dead_code)]
    clock: Arc<dyn Clock>,
}

impl PlexSyncRequester {
    pub fn new(
        watch_syncer: Option<PlexWatchSyncer>,
        label_syncer: Option<PlexLabelSyncer>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            watch_syncer,
            label_syncer,
            clock,
        }
    }
}

pub trait PlexSyncRequesterApi: Send + Sync {
    /// Moves the pending `kind` sync to now, or schedules one now when none
    /// is pending; `NotConfigured` without Plex.
    fn request(&self, kind: PlexSyncKind) -> Result<(), RequestPlexSyncError>;
}

impl PlexSyncRequesterApi for PlexSyncRequester {
    fn request(&self, _kind: PlexSyncKind) -> Result<(), RequestPlexSyncError> {
        Ok(())
    }
}
