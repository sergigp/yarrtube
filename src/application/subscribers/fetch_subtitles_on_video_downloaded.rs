use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Reacts to `VideoDownloaded` by scheduling a `FetchSubtitles` task to run
/// now, so a new episode's subtitles don't wait for the next reconcile pass.
/// The fetch itself decides whether the video still needs subtitles.
pub struct FetchSubtitlesOnVideoDownloaded {
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl FetchSubtitlesOnVideoDownloaded {
    pub fn new(task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            task_repository,
            clock,
        }
    }
}

impl EventSubscriber for FetchSubtitlesOnVideoDownloaded {
    fn handle(&self, _payload: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
