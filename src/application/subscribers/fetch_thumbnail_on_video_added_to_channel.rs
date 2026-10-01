use crate::domain::channel::ChannelHandle;
use crate::domain::task::Task;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use tracing::debug;

#[derive(Debug, Deserialize)]
struct VideoAddedToChannelPayload {
    channel_id: String,
    video_id: String,
}

/// Reacts to `VideoAddedToChannel` by scheduling a `FetchThumbnail` task to
/// run now, so the reconcile pass that added the video never waits on
/// `yt-dlp`. Mirrors `DownloadVideoOnVideoAddedToChannel`, and is
/// registered before it so the thumbnail task gets the lower id.
pub struct FetchThumbnailOnVideoAddedToChannel {
    channel_repository: Arc<dyn ChannelRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl FetchThumbnailOnVideoAddedToChannel {
    pub fn new(
        channel_repository: Arc<dyn ChannelRepository>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            channel_repository,
            task_repository,
            clock,
            videos_path: videos_path.into(),
        }
    }
}

impl EventSubscriber for FetchThumbnailOnVideoAddedToChannel {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoAddedToChannelPayload = serde_json::from_str(payload)?;
        let channel_id = ChannelHandle::new(payload.channel_id.as_str())?;
        let Some(channel) = self.channel_repository.find(&channel_id)? else {
            debug!(channel_id = %channel_id, "channel no longer exists, skipping thumbnail scheduling");
            return Ok(());
        };

        let output_dir = Path::new(&self.videos_path).join(channel.path.as_str());
        self.task_repository.schedule(
            &Task::FetchThumbnail {
                video_id: payload.video_id,
                output_dir: output_dir.to_string_lossy().to_string(),
            },
            self.clock.now(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, VideoLimit};
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::shared::Quality;
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::infrastructure::repositories::sqlite_channel_repository::SqliteChannelRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    #[test]
    fn it_should_schedule_a_thumbnail_fetch() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.shared_connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let subscriber = FetchThumbnailOnVideoAddedToChannel::new(
            channel_repository,
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        );

        let result = handle(
            &subscriber,
            r#"{"channel_id": "@somechannel", "video_id": "rec1"}"#,
        );

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchThumbnail {
                    video_id: "rec1".to_string(),
                    output_dir: "/videos/creators/somechannel".to_string(),
                },
            )]
        );
    }

    #[test]
    fn it_should_skip_if_channel_no_longer_exists() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let subscriber = FetchThumbnailOnVideoAddedToChannel::new(
            Arc::new(SqliteChannelRepository::new(db.shared_connection())),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        );

        let result = handle(
            &subscriber,
            r#"{"channel_id": "@missing", "video_id": "rec1"}"#,
        );

        assert_eq!(result, Ok(()));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    fn channel(channel_handle: &str) -> Channel {
        Channel::create(
            ChannelHandle::new(channel_handle).unwrap(),
            "Some Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
            fixed_timestamp(),
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
        subscriber: &FetchThumbnailOnVideoAddedToChannel,
        payload: &str,
    ) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
