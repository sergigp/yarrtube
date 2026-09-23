use crate::domain::channel::ChannelHandle;
use crate::domain::task::Task;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct ChannelDeletedPayload {
    channel_id: String,
    path: String,
}

/// Reacts to `ChannelDeleted` by scheduling a `DeleteChannelFiles` task to
/// run now — mirrors `DeletePlaylistFilesOnPlaylistDeleted`. The channel row
/// (and its `ChannelVideo`/`Video` rows) is already gone by the time this
/// runs, so its whole output directory is removed unconditionally.
pub struct DeleteChannelFilesOnChannelDeleted {
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl DeleteChannelFilesOnChannelDeleted {
    pub fn new(task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            task_repository,
            clock,
        }
    }
}

impl EventSubscriber for DeleteChannelFilesOnChannelDeleted {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: ChannelDeletedPayload = serde_json::from_str(payload)?;
        let Ok(_channel_id) = ChannelHandle::new(payload.channel_id.as_str()) else {
            return Ok(());
        };

        self.task_repository.schedule(
            &Task::DeleteChannelFiles {
                channel_id: payload.channel_id,
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

    fn subscriber(task_repository: Arc<dyn TaskRepository>) -> DeleteChannelFilesOnChannelDeleted {
        DeleteChannelFilesOnChannelDeleted::new(
            task_repository,
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    #[test]
    fn it_should_schedule_a_delete_channel_files_task_with_the_right_payload() {
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(task_repository.clone());

        subscriber
            .handle(r#"{"channel_id": "@somechannel", "path": "creators/somechannel"}"#)
            .unwrap();

        let scheduled = task_repository.scheduled();
        assert_eq!(
            *scheduled,
            vec![(
                Task::DeleteChannelFiles {
                    channel_id: "@somechannel".to_string(),
                    path: "creators/somechannel".to_string(),
                },
                fixed_timestamp()
            )]
        );
    }

    #[test]
    fn it_should_no_op_when_the_payload_channel_id_is_invalid() {
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(task_repository.clone());

        subscriber
            .handle(r#"{"channel_id": "", "path": "creators/somechannel"}"#)
            .unwrap();

        assert!(task_repository.scheduled().is_empty());
    }
}
