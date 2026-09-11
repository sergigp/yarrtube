use crate::domain::task::ScheduledTask;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct TaskResponse {
    pub id: i64,
    pub task_type: String,
    pub status: String,
    pub retries: i64,
    pub run_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_error: Option<String>,
}

impl From<ScheduledTask> for TaskResponse {
    fn from(task: ScheduledTask) -> Self {
        Self {
            id: task.id,
            task_type: task.task_type,
            status: task.status.as_str().to_string(),
            retries: task.retries,
            run_at: task.run_at,
            created_at: task.created_at,
            last_error: task.last_error,
        }
    }
}
