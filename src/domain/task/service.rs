use super::ScheduledTask;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use std::sync::Arc;

/// Orchestrates read access to the task aggregate for external callers (e.g.
/// HTTP). Background task scheduling/execution goes through `TaskRepository`
/// directly via `VideoService`/`TaskExecutor`; this exists for queries.
#[derive(Clone)]
pub struct TaskService {
    task_repository: Arc<dyn TaskRepository>,
}

impl TaskService {
    pub fn new(task_repository: Arc<dyn TaskRepository>) -> Self {
        Self { task_repository }
    }

    pub fn list_non_completed(&self) -> anyhow::Result<Vec<ScheduledTask>> {
        self.task_repository.list_non_completed()
    }
}
