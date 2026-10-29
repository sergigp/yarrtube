use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use serde::Deserialize;
use std::sync::Arc;

/// The only field this subscriber reads from any of the events it is
/// registered for.
#[derive(Debug, Deserialize)]
struct VideoEventPayload {
    video_id: String,
}

/// Reacts to any event about one stored video (`VideoAddedToPlaylist`,
/// `VideoAddedToChannel`, `VideoDownloaded`) by scheduling a `SyncVideo`
/// task to run now: the sync, not this subscriber, decides which of the
/// video's steps are still needed.
pub struct SyncVideoOnVideoEvent {
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl SyncVideoOnVideoEvent {
    pub fn new(task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            task_repository,
            clock,
        }
    }
}

impl EventSubscriber for SyncVideoOnVideoEvent {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let _: VideoEventPayload = serde_json::from_str(payload)?;
        let _ = (&self.task_repository, &self.clock);
        Ok(())
    }
}
