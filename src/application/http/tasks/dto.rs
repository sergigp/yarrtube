use crate::domain::task::TaskView;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, PartialEq)]
pub struct TaskResponse {
    pub id: i64,
    pub task_type: String,
    pub status: String,
    pub retries: i64,
    pub run_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub payload: HashMap<String, String>,
}

impl From<TaskView> for TaskResponse {
    fn from(view: TaskView) -> Self {
        Self {
            id: view.id,
            task_type: view.task_type,
            status: view.status.as_str().to_string(),
            retries: view.retries,
            run_at: view.run_at,
            created_at: view.created_at,
            last_error: view.last_error,
            payload: view.payload,
        }
    }
}
