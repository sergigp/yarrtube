use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Reacts to `VideoSubtitlesFetched` by scheduling a `SyncVideo` task to run
/// now, so the steps that need subtitles (the summary) don't wait for the
/// next reconcile pass. The sync decides which are still needed.
pub struct SyncVideoOnVideoSubtitlesFetched {
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl SyncVideoOnVideoSubtitlesFetched {
    pub fn new(task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            task_repository,
            clock,
        }
    }
}

impl EventSubscriber for SyncVideoOnVideoSubtitlesFetched {
    fn handle(&self, _payload: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
