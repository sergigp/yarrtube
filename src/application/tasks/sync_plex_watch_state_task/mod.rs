use crate::domain::services::{PlexWatchSyncer, PlexWatchSyncerApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};

/// Runs one Plex watch sync (`PlexWatchSyncer::sync`), which requests the
/// next one itself. Registered only when the Plex integration is configured.
pub struct SyncPlexWatchStateTask {
    plex_watch_syncer: PlexWatchSyncer,
}

impl SyncPlexWatchStateTask {
    pub fn new(plex_watch_syncer: PlexWatchSyncer) -> Self {
        Self { plex_watch_syncer }
    }
}

impl TaskHandler for SyncPlexWatchStateTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::SyncPlexWatchState = task else {
            return Err(unexpected_task(task));
        };
        self.plex_watch_syncer.sync().map(|()| TaskOutcome::Done)
    }
}

#[cfg(test)]
mod tests;
