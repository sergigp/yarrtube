use crate::domain::plex_status::{PlexSyncKind, RequestPlexSyncError};
use crate::domain::services::{
    PlexLabelSyncer, PlexLabelSyncerApi, PlexWatchSyncer, PlexWatchSyncerApi,
};
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Makes a Plex sync due now.
#[derive(Clone)]
pub struct PlexSyncRequester {
    watch_syncer: Option<PlexWatchSyncer>,
    label_syncer: Option<PlexLabelSyncer>,
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
    fn request(&self, kind: PlexSyncKind) -> Result<(), RequestPlexSyncError> {
        let now = self.clock.now();
        match kind {
            PlexSyncKind::WatchState => self.watch_syncer.as_ref().map(|s| s.request_sync(now)),
            PlexSyncKind::Labels => self.label_syncer.as_ref().map(|s| s.request_sync(now)),
        }
        .ok_or(RequestPlexSyncError::NotConfigured)?
        .map_err(RequestPlexSyncError::Repository)
    }
}
