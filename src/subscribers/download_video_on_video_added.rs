use crate::domain::shared::{PlaylistId, VideoId};
use crate::domain::task::Task;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::system_clock::Clock;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct VideoAddedPayload {
    playlist_id: String,
    video_id: String,
}

/// Reacts to `VideoAdded` by scheduling a `DownloadVideo` task to run now,
/// rather than downloading inline — downloading is slow and external, so it
/// belongs on the task queue to get retry/backoff/dead-letter for free.
pub struct DownloadVideoOnVideoAdded {
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl DownloadVideoOnVideoAdded {
    pub fn new(task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            task_repository,
            clock,
        }
    }
}

impl EventSubscriber for DownloadVideoOnVideoAdded {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoAddedPayload = serde_json::from_str(payload)?;
        if PlaylistId::new(payload.playlist_id.as_str()).is_err()
            || VideoId::new(payload.video_id.as_str()).is_err()
        {
            return Ok(());
        }

        self.task_repository.schedule(
            &Task::DownloadVideo {
                playlist_id: payload.playlist_id,
                video_id: payload.video_id,
            },
            self.clock.now(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn subscriber(task_repository: Arc<dyn TaskRepository>) -> DownloadVideoOnVideoAdded {
        DownloadVideoOnVideoAdded::new(task_repository, Arc::new(FixedClock(fixed_timestamp())))
    }

    #[test]
    fn it_should_schedule_a_download_video_task_for_a_valid_payload() {
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(task_repository.clone());

        subscriber
            .handle(r#"{"playlist_id": "PL1", "video_id": "vid1"}"#)
            .unwrap();

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert_eq!(
            *scheduled,
            vec![(
                Task::DownloadVideo {
                    playlist_id: "PL1".to_string(),
                    video_id: "vid1".to_string(),
                },
                fixed_timestamp()
            )]
        );
    }

    #[test]
    fn it_should_no_op_when_the_payload_ids_are_invalid() {
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(task_repository.clone());

        subscriber
            .handle(r#"{"playlist_id": "", "video_id": "vid1"}"#)
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }
}
