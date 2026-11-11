use crate::domain::task::Task;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use serde::Deserialize;
use std::sync::Arc;

/// The only field this subscriber reads from `VideoSummaryGenerated`.
#[derive(Debug, Deserialize)]
struct VideoSummaryGeneratedPayload {
    video_id: String,
}

/// Reacts to `VideoSummaryGenerated` by scheduling a `SyncVideo` task to run
/// now, so the steps that need a summary (the categorization) don't wait for
/// the next reconcile pass. The sync decides which are still needed.
pub struct SyncVideoOnVideoSummaryGenerated {
    task_repository: Arc<dyn TaskRepository>,
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
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoSummaryGeneratedPayload = serde_json::from_str(payload)?;
        self.task_repository.schedule(
            &Task::SyncVideo {
                video_id: payload.video_id,
            },
            self.clock.now(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    #[test]
    fn it_should_schedule_a_video_sync() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let subscriber = SyncVideoOnVideoSummaryGenerated::new(
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let result = subscriber.handle(r#"{"video_id": "rec1"}"#);

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        let scheduled: Vec<(Task, DateTime<Utc>)> = task_repository
            .list_non_completed()
            .unwrap()
            .into_iter()
            .map(|task| (task.task, task.run_at))
            .collect();
        assert_eq!(
            scheduled,
            vec![(
                Task::SyncVideo {
                    video_id: "rec1".to_string(),
                },
                fixed_timestamp()
            )]
        );
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }
}
