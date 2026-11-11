use crate::domain::services::{PlexLabelSyncer, PlexLabelSyncerApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};

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
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::SyncPlexLabels = task else {
            return Err(unexpected_task(task));
        };
        self.plex_label_syncer.sync().map(|()| TaskOutcome::Done)
    }
}

#[cfg(test)]
mod tests;
