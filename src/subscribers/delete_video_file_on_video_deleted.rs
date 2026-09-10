use crate::domain::shared::{PlaylistId, VideoId};
use crate::domain::task::Task;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct VideoDeletedPayload {
    playlist_id: String,
    video_id: String,
    title: String,
    was_downloaded: bool,
}

/// Reacts to `VideoDeleted` by scheduling a `DeleteVideoFile` task to run
/// now, only when `was_downloaded` — a video that never finished
/// downloading has no file to clean up. Unlike
/// `DownloadVideoOnVideoAdded`, no repository lookup is needed here: the
/// task handler resolves the playlist itself.
pub struct DeleteVideoFileOnVideoDeleted {
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl DeleteVideoFileOnVideoDeleted {
    pub fn new(task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            task_repository,
            clock,
        }
    }
}

impl EventSubscriber for DeleteVideoFileOnVideoDeleted {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoDeletedPayload = serde_json::from_str(payload)?;
        let (Ok(_playlist_id), Ok(_video_id)) = (
            PlaylistId::new(payload.playlist_id.as_str()),
            VideoId::new(payload.video_id.as_str()),
        ) else {
            return Ok(());
        };

        if !payload.was_downloaded {
            return Ok(());
        }

        self.task_repository.schedule(
            &Task::DeleteVideoFile {
                playlist_id: payload.playlist_id,
                video_id: payload.video_id,
                title: payload.title,
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

    fn subscriber(task_repository: Arc<dyn TaskRepository>) -> DeleteVideoFileOnVideoDeleted {
        DeleteVideoFileOnVideoDeleted::new(task_repository, Arc::new(FixedClock(fixed_timestamp())))
    }

    #[test]
    fn it_should_schedule_a_delete_video_file_task_when_the_video_was_downloaded() {
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(task_repository.clone());

        subscriber
            .handle(
                r#"{"playlist_id": "PL1", "video_id": "vid1", "title": "My Video", "was_downloaded": true}"#,
            )
            .unwrap();

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert_eq!(
            *scheduled,
            vec![(
                Task::DeleteVideoFile {
                    playlist_id: "PL1".to_string(),
                    video_id: "vid1".to_string(),
                    title: "My Video".to_string(),
                },
                fixed_timestamp()
            )]
        );
    }

    #[test]
    fn it_should_not_schedule_a_task_when_the_video_was_not_downloaded() {
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(task_repository.clone());

        subscriber
            .handle(
                r#"{"playlist_id": "PL1", "video_id": "vid1", "title": "My Video", "was_downloaded": false}"#,
            )
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_no_op_when_the_payload_ids_are_invalid() {
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(task_repository.clone());

        subscriber
            .handle(
                r#"{"playlist_id": "", "video_id": "vid1", "title": "My Video", "was_downloaded": true}"#,
            )
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }
}
