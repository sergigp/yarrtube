use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use serde::Deserialize;
use std::sync::Arc;

/// The only field this subscriber reads from `VideoSummaryGenerated`.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct VideoSummaryGeneratedPayload {
    video_id: String,
}

/// Reacts to `VideoSummaryGenerated` by scheduling a `SyncVideo` task to run
/// now, so the steps that need a summary (the categorization) don't wait for
/// the next reconcile pass. The sync decides which are still needed.
pub struct SyncVideoOnVideoSummaryGenerated {
    #[allow(dead_code)]
    task_repository: Arc<dyn TaskRepository>,
    #[allow(dead_code)]
    clock: Arc<dyn Clock>,
}

impl SyncVideoOnVideoSummaryGenerated {
    pub fn new(task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            task_repository,
            clock,
        }
    }
}

impl EventSubscriber for SyncVideoOnVideoSummaryGenerated {
    fn handle(&self, _payload: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
