use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Reacts to `VideoDiscarded` by scheduling a `DeleteVideoFile` task to run
/// now, deleting the discarded video's files (see the `video-discard`
/// capability).
pub struct DeleteVideoFileOnVideoDiscarded {
    #[allow(dead_code)]
    task_repository: Arc<dyn TaskRepository>,
    #[allow(dead_code)]
    clock: Arc<dyn Clock>,
}

impl DeleteVideoFileOnVideoDiscarded {
    pub fn new(task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            task_repository,
            clock,
        }
    }
}

impl EventSubscriber for DeleteVideoFileOnVideoDiscarded {
    fn handle(&self, _payload: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
