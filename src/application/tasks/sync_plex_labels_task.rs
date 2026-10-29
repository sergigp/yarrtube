use crate::domain::services::{PlexLabelSyncer, PlexLabelSyncerApi};
use crate::domain::task::Task;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Runs one Plex label sync (`PlexLabelSyncer::sync`), which requests the
/// next one itself. Registered only when the Plex integration is configured.
pub struct SyncPlexLabelsTask {
    plex_label_syncer: PlexLabelSyncer,
}

impl SyncPlexLabelsTask {
    pub fn new(plex_label_syncer: PlexLabelSyncer) -> Self {
        Self { plex_label_syncer }
    }
}

impl TaskHandler for SyncPlexLabelsTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        Task::decode_sync_plex_labels_payload(payload)?;
        self.plex_label_syncer.sync()
    }
}
