use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

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
    fn handle(&self, _payload: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
