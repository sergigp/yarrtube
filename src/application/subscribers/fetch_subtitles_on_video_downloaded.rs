use crate::domain::task::Task;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use serde::Deserialize;
use std::sync::Arc;

/// The event's `folder` is ignored: the fetch names the file from the
/// video's episode number.
#[derive(Debug, Deserialize)]
struct VideoDownloadedPayload {
    video_id: String,
    output_dir: String,
}

/// Reacts to `VideoDownloaded` by scheduling a `FetchSubtitles` task to run
/// now, so a new episode's subtitles don't wait for the next reconcile pass.
/// The fetch itself decides whether the video still needs subtitles.
pub struct FetchSubtitlesOnVideoDownloaded {
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl FetchSubtitlesOnVideoDownloaded {
    pub fn new(task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            task_repository,
            clock,
        }
    }
}

impl EventSubscriber for FetchSubtitlesOnVideoDownloaded {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoDownloadedPayload = serde_json::from_str(payload)?;
        self.task_repository.schedule(
            &Task::FetchSubtitles {
                video_id: payload.video_id,
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
    fn it_should_schedule_a_subtitle_fetch_when_a_video_is_downloaded() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let subscriber = FetchSubtitlesOnVideoDownloaded::new(
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let result = handle(&subscriber, &payload("rec1"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(1, &fetch_subtitles("rec1"))]
        );
    }

    #[test]
    fn it_should_not_schedule_a_second_subtitle_fetch_if_one_is_queued() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        task_repository
            .schedule(&fetch_subtitles("rec1"), fixed_timestamp())
            .unwrap();
        let subscriber = FetchSubtitlesOnVideoDownloaded::new(
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let result = handle(&subscriber, &payload("rec1"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(1, &fetch_subtitles("rec1"))]
        );
    }

    fn fetch_subtitles(video_id: &str) -> Task {
        Task::FetchSubtitles {
            video_id: video_id.to_string(),
            output_dir: "/videos/creators/somechannel".to_string(),
        }
    }

    fn payload(video_id: &str) -> String {
        serde_json::json!({
            "video_id": video_id,
            "output_dir": "/videos/creators/somechannel",
            "folder": "Season 2026",
        })
        .to_string()
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

    fn handle(subscriber: &FetchSubtitlesOnVideoDownloaded, payload: &str) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
