use crate::domain::video::{Video, VideoRecordId, VideoTarget};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Owns a stored video's lifecycle — see the `video-sync` capability:
/// decides from the video's stored state which of its steps are still
/// needed and schedules each as its own task.
#[derive(Clone)]
pub struct VideoReconciler {
    video_repository: Arc<dyn VideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl VideoReconciler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_repository,
            channel_video_repository,
            playlist_video_repository,
            channel_repository,
            playlist_repository,
            task_repository,
            clock,
            videos_path: videos_path.into(),
        }
    }
}

pub trait VideoReconcilerApi: Send + Sync {
    /// Finds the video and the playlist or channel holding it, then
    /// schedules every step its stored state still needs. No-op if either
    /// is gone.
    fn reconcile(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;

    /// Schedules every step still needed by `video`, already read along with
    /// its owner's `target` (a reconcile pass).
    fn reconcile_video(&self, video: &Video, target: &VideoTarget) -> anyhow::Result<()>;

    /// Schedules only the download of `video`, which a reconcile pass just
    /// reset for redownload.
    fn schedule_download(&self, video: &Video, target: &VideoTarget) -> anyhow::Result<()>;
}

impl VideoReconcilerApi for VideoReconciler {
    fn reconcile(&self, video_id: &VideoRecordId) -> anyhow::Result<()> {
        let _ = video_id;
        Ok(())
    }

    fn reconcile_video(&self, video: &Video, target: &VideoTarget) -> anyhow::Result<()> {
        let _ = (video, target);
        Ok(())
    }

    fn schedule_download(&self, video: &Video, target: &VideoTarget) -> anyhow::Result<()> {
        let _ = (video, target);
        Ok(())
    }
}
