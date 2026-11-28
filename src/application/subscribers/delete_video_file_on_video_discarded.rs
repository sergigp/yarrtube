use crate::domain::task::Task;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::domain_events::event_subscriber::EventSubscriber;
use crate::infrastructure::shared::system_clock::Clock;
use serde::Deserialize;
use std::sync::Arc;

/// The fields this subscriber reads from `VideoDiscarded`.
#[derive(Debug, Deserialize)]
struct VideoDiscardedPayload {
    output_dir: String,
    filename: Option<String>,
    thumbnail_filename: Option<String>,
    subtitle_filename: Option<String>,
}

/// Reacts to `VideoDiscarded` by scheduling a `DeleteVideoFile` task to run
/// now, deleting the discarded video's files (see the `video-discard`
/// capability).
pub struct DeleteVideoFileOnVideoDiscarded {
    task_repository: Arc<dyn TaskRepository>,
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
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoDiscardedPayload = serde_json::from_str(payload)?;
        self.task_repository.schedule(
            &Task::DeleteVideoFile {
                filename: payload.filename,
                thumbnail_filename: payload.thumbnail_filename,
                subtitle_filename: payload.subtitle_filename,
                output_dir: payload.output_dir,
            },
            self.clock.now(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    #[test]
    fn it_should_schedule_the_deletion_of_the_discarded_video_files() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let subscriber = DeleteVideoFileOnVideoDiscarded::new(
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let result = subscriber
            .handle(
                r#"{"video_id": "rec1", "output_dir": "/videos/creators/somechannel", "filename": "Season 2026/S2026E031501 - My Video.mp4", "thumbnail_filename": "Season 2026/S2026E031501 - My Video.jpg", "subtitle_filename": "Season 2026/S2026E031501 - My Video.en.srt"}"#,
            )
            .map_err(|e| e.to_string());

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![ScheduledTask {
                id: 1,
                task: Task::DeleteVideoFile {
                    filename: Some("Season 2026/S2026E031501 - My Video.mp4".to_string()),
                    thumbnail_filename: Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                    subtitle_filename: Some(
                        "Season 2026/S2026E031501 - My Video.en.srt".to_string()
                    ),
                    output_dir: "/videos/creators/somechannel".to_string(),
                },
                status: TaskStatus::Pending,
                retries: 0,
                run_at: fixed_timestamp(),
                created_at: fixed_timestamp(),
                updated_at: fixed_timestamp(),
                last_error: None,
            }]
        );
    }

    #[test]
    fn it_should_schedule_the_deletion_of_a_discarded_thumbnail() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let subscriber = DeleteVideoFileOnVideoDiscarded::new(
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let result = subscriber
            .handle(
                r#"{"video_id": "rec1", "output_dir": "/videos/creators/somechannel", "filename": null, "thumbnail_filename": "Season 2026/S2026E031601 - My Video.jpg", "subtitle_filename": null}"#,
            )
            .map_err(|e| e.to_string());

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![ScheduledTask {
                id: 1,
                task: Task::DeleteVideoFile {
                    filename: None,
                    thumbnail_filename: Some("Season 2026/S2026E031601 - My Video.jpg".to_string()),
                    subtitle_filename: None,
                    output_dir: "/videos/creators/somechannel".to_string(),
                },
                status: TaskStatus::Pending,
                retries: 0,
                run_at: fixed_timestamp(),
                created_at: fixed_timestamp(),
                updated_at: fixed_timestamp(),
                last_error: None,
            }]
        );
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }
}
