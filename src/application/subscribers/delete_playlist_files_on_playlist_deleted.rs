use crate::domain::playlist::PlaylistId;
use crate::domain::task::Task;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct PlaylistDeletedPayload {
    playlist_id: String,
    path: String,
}

/// Reacts to `PlaylistDeleted` by scheduling a `DeletePlaylistFiles` task to
/// run now. Unlike `DeleteVideoFileOnVideoDeleted`, no lookup or downloaded
/// condition applies: the playlist row is already gone, and its whole
/// output directory is removed unconditionally.
pub struct DeletePlaylistFilesOnPlaylistDeleted {
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl DeletePlaylistFilesOnPlaylistDeleted {
    pub fn new(task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            task_repository,
            clock,
        }
    }
}

impl EventSubscriber for DeletePlaylistFilesOnPlaylistDeleted {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: PlaylistDeletedPayload = serde_json::from_str(payload)?;
        let Ok(_playlist_id) = PlaylistId::new(payload.playlist_id.as_str()) else {
            return Ok(());
        };

        self.task_repository.schedule(
            &Task::DeletePlaylistFiles {
                playlist_id: payload.playlist_id,
                path: payload.path,
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
    use rusqlite::Connection;
    use std::sync::Mutex;

    #[test]
    fn it_should_schedule_playlist_files_deletion() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let subscriber = DeletePlaylistFilesOnPlaylistDeleted::new(
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let result = handle(
            &subscriber,
            r#"{"playlist_id": "PL1", "path": "music/chill"}"#,
        );

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::DeletePlaylistFiles {
                    playlist_id: "PL1".to_string(),
                    path: "music/chill".to_string(),
                },
            )]
        );
    }

    #[test]
    fn it_should_skip_if_invalid_playlist_id_provided() {
        let result = handle(
            &any_subscriber(),
            r#"{"playlist_id": "", "path": "music/chill"}"#,
        );

        assert_eq!(result, Ok(()));
    }

    /// A subscriber for tests whose payload is rejected before scheduling
    /// anything. Its task repository sits on an unmigrated in-memory
    /// database, so a payload that wrongly got through would fail loudly
    /// instead of passing.
    fn any_subscriber() -> DeletePlaylistFilesOnPlaylistDeleted {
        DeletePlaylistFilesOnPlaylistDeleted::new(
            Arc::new(SqliteTaskRepository::new(
                Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
        )
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

    fn handle(
        subscriber: &DeletePlaylistFilesOnPlaylistDeleted,
        payload: &str,
    ) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
