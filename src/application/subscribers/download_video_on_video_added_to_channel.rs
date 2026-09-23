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

/// Reacts to `VideoAddedToChannel` by scheduling a `DownloadVideo` task to
/// run now, mirroring `DownloadVideoOnVideoAddedToPlaylist` against
/// `ChannelRepository` instead of `PlaylistRepository`.
pub struct DownloadVideoOnVideoAddedToChannel {
    channel_repository: Arc<dyn ChannelRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl DownloadVideoOnVideoAddedToChannel {
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

impl EventSubscriber for DownloadVideoOnVideoAddedToChannel {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoAddedToChannelPayload = serde_json::from_str(payload)?;
        let Ok(channel_id) = ChannelHandle::new(payload.channel_id.as_str()) else {
            return Ok(());
        };

        let Some(channel) = self.channel_repository.find(&channel_id)? else {
            debug!(channel_id = %channel_id, "channel no longer exists, skipping download scheduling");
            return Ok(());
        };

        let output_dir = Path::new(&self.videos_path).join(channel.path.as_str());
        self.task_repository.schedule(
            &Task::DownloadVideo {
                video_id: payload.video_id,
                quality: channel.quality.as_str().to_string(),
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
    use crate::domain::task::Task;
    use crate::infrastructure::repositories::sqlite_channel_repository::FakeChannelRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn channel_repository_with(id: &str, quality: Quality) -> Arc<FakeChannelRepository> {
        let repository = Arc::new(FakeChannelRepository::default());
        repository
            .insert(&Channel::create(
                ChannelHandle::new(id).unwrap(),
                "Some Channel",
                "UC123",
                quality,
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
    ) -> DownloadVideoOnVideoAddedToChannel {
        DownloadVideoOnVideoAddedToChannel::new(
            channel_repository,
            task_repository,
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        )
    }

    #[test]
    fn it_should_schedule_a_download_video_task_with_the_channels_quality_and_output_dir() {
        let channel_repository = channel_repository_with("@somechannel", Quality::Low);
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(channel_repository, task_repository.clone());

        subscriber
            .handle(r#"{"channel_id": "@somechannel", "video_id": "rec1"}"#)
            .unwrap();

        let scheduled = task_repository.scheduled();
        assert_eq!(
            *scheduled,
            vec![(
                Task::DownloadVideo {
                    video_id: "rec1".to_string(),
                    quality: "low".to_string(),
                    output_dir: "/videos/creators/somechannel".to_string(),
                },
                fixed_timestamp()
            )]
        );
    }

    #[test]
    fn it_should_no_op_when_the_payload_channel_id_is_invalid() {
        let channel_repository = channel_repository_with("@somechannel", Quality::High);
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(channel_repository, task_repository.clone());

        subscriber
            .handle(r#"{"channel_id": "", "video_id": "rec1"}"#)
            .unwrap();

        assert!(task_repository.scheduled().is_empty());
    }

    #[test]
    fn it_should_no_op_when_the_channel_no_longer_exists() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(channel_repository, task_repository.clone());

        subscriber
            .handle(r#"{"channel_id": "@missing", "video_id": "rec1"}"#)
            .unwrap();

        assert!(task_repository.scheduled().is_empty());
    }
}
