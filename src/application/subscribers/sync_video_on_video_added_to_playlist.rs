use crate::domain::task::Task;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use serde::Deserialize;
use std::sync::Arc;

/// The only field this subscriber reads from `VideoAddedToPlaylist`.
#[derive(Debug, Deserialize)]
struct VideoAddedToPlaylistPayload {
    video_id: String,
}

/// Reacts to `VideoAddedToPlaylist` by scheduling a `SyncVideo` task to run
/// now: the sync, not this subscriber, decides which of the new video's
/// steps are needed.
pub struct SyncVideoOnVideoAddedToPlaylist {
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl SyncVideoOnVideoAddedToPlaylist {
    pub fn new(task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            task_repository,
            clock,
        }
    }
}

impl EventSubscriber for SyncVideoOnVideoAddedToPlaylist {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoAddedToPlaylistPayload = serde_json::from_str(payload)?;
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
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    #[test]
    fn it_should_schedule_a_video_sync_when_a_video_is_added_to_a_playlist() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let subscriber = SyncVideoOnVideoAddedToPlaylist::new(
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let result = handle(&subscriber, r#"{"playlist_id": "PL1", "video_id": "rec1"}"#);

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(1, &sync_video("rec1"))]
        );
    }

    fn sync_video(video_id: &str) -> Task {
        Task::SyncVideo {
            video_id: video_id.to_string(),
        }
    }

    fn pending_task(id: i64, task: &Task) -> ScheduledTask {
        ScheduledTask {
            id,
            task_type: task.task_type().to_string(),
            payload: task.payload().to_string(),
            status: TaskStatus::Pending,
            retries: 0,
            run_at: fixed_timestamp(),
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn handle(subscriber: &SyncVideoOnVideoAddedToPlaylist, payload: &str) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
