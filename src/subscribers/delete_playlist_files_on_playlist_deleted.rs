use crate::domain::shared::PlaylistId;
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
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn subscriber(
        task_repository: Arc<dyn TaskRepository>,
    ) -> DeletePlaylistFilesOnPlaylistDeleted {
        DeletePlaylistFilesOnPlaylistDeleted::new(
            task_repository,
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    #[test]
    fn it_should_schedule_a_delete_playlist_files_task_with_the_right_payload() {
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(task_repository.clone());

        subscriber
            .handle(r#"{"playlist_id": "PL1", "path": "music/chill"}"#)
            .unwrap();

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert_eq!(
            *scheduled,
            vec![(
                Task::DeletePlaylistFiles {
                    playlist_id: "PL1".to_string(),
                    path: "music/chill".to_string(),
                },
                fixed_timestamp()
            )]
        );
    }

    #[test]
    fn it_should_no_op_when_the_payload_playlist_id_is_invalid() {
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(task_repository.clone());

        subscriber
            .handle(r#"{"playlist_id": "", "path": "music/chill"}"#)
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }
}
