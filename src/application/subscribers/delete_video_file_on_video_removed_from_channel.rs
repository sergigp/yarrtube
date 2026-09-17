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
struct VideoRemovedFromChannelPayload {
    channel_id: String,
    filename: Option<String>,
    thumbnail_filename: Option<String>,
    was_downloaded: bool,
}

/// Reacts to `VideoRemovedFromChannel` by scheduling a `DeleteVideoFile`
/// task to run now, mirroring `DeleteVideoFileOnVideoRemovedFromPlaylist`
/// against `ChannelRepository` instead of `PlaylistRepository`.
pub struct DeleteVideoFileOnVideoRemovedFromChannel {
    channel_repository: Arc<dyn ChannelRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl DeleteVideoFileOnVideoRemovedFromChannel {
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

impl EventSubscriber for DeleteVideoFileOnVideoRemovedFromChannel {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoRemovedFromChannelPayload = serde_json::from_str(payload)?;
        let Ok(channel_id) = ChannelHandle::new(payload.channel_id.as_str()) else {
            return Ok(());
        };

        if !payload.was_downloaded {
            return Ok(());
        }

        let Some(channel) = self.channel_repository.find(&channel_id)? else {
            debug!(channel_id = %channel_id, "channel no longer exists, skipping file deletion scheduling");
            return Ok(());
        };

        let output_dir = Path::new(&self.videos_path).join(channel.path.as_str());
        self.task_repository.schedule(
            &Task::DeleteVideoFile {
                filename: payload.filename,
                thumbnail_filename: payload.thumbnail_filename,
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
    use crate::infrastructure::repositories::sqlite_channel_repository::FakeChannelRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn channel_repository_with(id: &str) -> Arc<FakeChannelRepository> {
        let repository = Arc::new(FakeChannelRepository::default());
        repository
            .insert(&Channel::create(
                ChannelHandle::new(id).unwrap(),
                "Some Channel",
                "UC123",
                Quality::High,
                VideoLimit::new(10).unwrap(),
                PlaylistPath::new("creators/somechannel").unwrap(),
                None,
                fixed_timestamp(),
            ))
            .unwrap();
        repository
    }

    fn subscriber(
        channel_repository: Arc<dyn ChannelRepository>,
        task_repository: Arc<dyn TaskRepository>,
    ) -> DeleteVideoFileOnVideoRemovedFromChannel {
        DeleteVideoFileOnVideoRemovedFromChannel::new(
            channel_repository,
            task_repository,
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        )
    }

    #[test]
    fn it_should_schedule_a_delete_video_file_task_when_the_video_was_downloaded() {
        let channel_repository = channel_repository_with("@somechannel");
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(channel_repository, task_repository.clone());

        subscriber
            .handle(
                r#"{"channel_id": "@somechannel", "video_id": "rec1", "title": "My Video", "filename": "My Video.mp4", "thumbnail_filename": "My Video.jpg", "was_downloaded": true}"#,
            )
            .unwrap();

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert_eq!(
            *scheduled,
            vec![(
                Task::DeleteVideoFile {
                    filename: Some("My Video.mp4".to_string()),
                    thumbnail_filename: Some("My Video.jpg".to_string()),
                    output_dir: "/videos/creators/somechannel".to_string(),
                },
                fixed_timestamp()
            )]
        );
    }

    #[test]
    fn it_should_not_schedule_a_task_when_the_video_was_not_downloaded() {
        let channel_repository = channel_repository_with("@somechannel");
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(channel_repository, task_repository.clone());

        subscriber
            .handle(
                r#"{"channel_id": "@somechannel", "video_id": "rec1", "title": "My Video", "filename": null, "thumbnail_filename": null, "was_downloaded": false}"#,
            )
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_no_op_when_the_payload_channel_id_is_invalid() {
        let channel_repository = channel_repository_with("@somechannel");
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(channel_repository, task_repository.clone());

        subscriber
            .handle(
                r#"{"channel_id": "", "video_id": "rec1", "title": "My Video", "filename": null, "thumbnail_filename": null, "was_downloaded": true}"#,
            )
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_no_op_when_the_channel_no_longer_exists() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(channel_repository, task_repository.clone());

        subscriber
            .handle(
                r#"{"channel_id": "@missing", "video_id": "rec1", "title": "My Video", "filename": "My Video.mp4", "thumbnail_filename": null, "was_downloaded": true}"#,
            )
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }
}
