use super::scheduled_task::TaskStatus;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// A task enriched for display, with whatever context `TaskViewSearcher`
/// could resolve from the task's stored payload. `payload`'s keys vary by
/// `task_type`; a key is omitted when it can't be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskView {
    pub id: i64,
    pub task_type: String,
    pub status: TaskStatus,
    pub retries: i64,
    pub run_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub payload: HashMap<String, String>,
}
