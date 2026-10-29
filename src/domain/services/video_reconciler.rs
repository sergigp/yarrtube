use crate::domain::shared::Quality;
use crate::domain::video::{
    Video, VideoRecordId, VideoSyncPlan, VideoSyncStep, VideoTarget, resolve_output_dir,
};
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
        let Some(video) = self.video_repository.find(video_id)? else {
            return Ok(());
        };
        let Some(target) = self.find_target(&video)? else {
            return Ok(());
        };
        self.reconcile_video(&video, &target)
    }

    fn reconcile_video(&self, video: &Video, target: &VideoTarget) -> anyhow::Result<()> {
        VideoSyncPlan::for_video(video)
            .steps
            .iter()
            .try_for_each(|step| self.schedule(*step, video, target))
    }

    fn schedule_download(&self, video: &Video, target: &VideoTarget) -> anyhow::Result<()> {
        self.schedule(VideoSyncStep::Download, video, target)
    }
}

impl VideoReconciler {
    fn schedule(
        &self,
        step: VideoSyncStep,
        video: &Video,
        target: &VideoTarget,
    ) -> anyhow::Result<()> {
        self.task_repository
            .schedule(&step.task_for(video, target), self.clock.now())
    }

    /// The target of the channel holding `video`, else of the playlist
    /// holding it; `None` when neither is found.
    fn find_target(&self, video: &Video) -> anyhow::Result<Option<VideoTarget>> {
        match self.find_channel_target(video)? {
            Some(target) => Ok(Some(target)),
            None => self.find_playlist_target(video),
        }
    }

    fn find_channel_target(&self, video: &Video) -> anyhow::Result<Option<VideoTarget>> {
        let Some(channel_video) = self.channel_video_repository.find_by_video(&video.id)? else {
            return Ok(None);
        };
        Ok(self
            .channel_repository
            .find(&channel_video.channel_id)?
            .map(|channel| self.target(channel.path.as_str(), channel.quality)))
    }

    fn find_playlist_target(&self, video: &Video) -> anyhow::Result<Option<VideoTarget>> {
        let Some(playlist_video) = self.playlist_video_repository.find_by_video(&video.id)? else {
            return Ok(None);
        };
        Ok(self
            .playlist_repository
            .find(&playlist_video.playlist_id)?
            .map(|playlist| self.target(playlist.path.as_str(), playlist.quality)))
    }

    fn target(&self, path: &str, quality: Quality) -> VideoTarget {
        VideoTarget {
            output_dir: resolve_output_dir(&self.videos_path, path),
            quality,
        }
    }
}
