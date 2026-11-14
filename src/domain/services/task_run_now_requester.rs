use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Brings pending tasks forward so they run now — see the `task-scheduling`
/// capability.
#[derive(Clone)]
pub struct TaskRunNowRequester {
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl TaskRunNowRequester {
    pub fn new(task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            task_repository,
            clock,
        }
    }
}

pub trait TaskRunNowRequesterApi: Send + Sync {
    /// Each pending task among `ids` becomes due now; others are ignored.
    fn run_now(&self, ids: &[i64]) -> anyhow::Result<()>;
}

impl TaskRunNowRequesterApi for TaskRunNowRequester {
    fn run_now(&self, ids: &[i64]) -> anyhow::Result<()> {
        let now = self.clock.now();
        self.task_repository
            .list_pending_by_ids(ids)?
            .into_iter()
            .try_for_each(|task| self.task_repository.update(&task.bring_forward(now, now)))
    }
}
