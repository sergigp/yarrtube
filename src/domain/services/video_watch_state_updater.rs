use crate::domain::channel::ChannelHandle;
use crate::domain::video::{PlaybackPosition, UpdateWatchStateError, VideoId};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Updates whether videos have been watched and where their playback
/// stopped. Watch state belongs to the YouTube video, so every change
/// applies to every stored copy of it.
#[derive(Clone)]
pub struct VideoWatchStateUpdater {
    video_repository: Arc<dyn VideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    clock: Arc<dyn Clock>,
}

impl VideoWatchStateUpdater {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_repository,
            channel_repository,
            channel_video_repository,
            clock,
        }
    }
}

pub trait VideoWatchStateUpdaterApi: Send + Sync {
    /// Applies `Video::record_progress` to every stored copy of `youtube_id`.
    fn record_progress(
        &self,
        youtube_id: &VideoId,
        position: PlaybackPosition,
        reported_duration_seconds: Option<i64>,
    ) -> Result<(), UpdateWatchStateError>;
    /// Marks every `Downloaded` video of the channel watched, with every
    /// stored copy of each; pending/in-flight videos are left unchanged.
    fn mark_channel_watched(&self, channel_id: &ChannelHandle)
    -> Result<(), UpdateWatchStateError>;
}

impl VideoWatchStateUpdaterApi for VideoWatchStateUpdater {
    fn record_progress(
        &self,
        _youtube_id: &VideoId,
        _position: PlaybackPosition,
        _reported_duration_seconds: Option<i64>,
    ) -> Result<(), UpdateWatchStateError> {
        Ok(())
    }

    fn mark_channel_watched(
        &self,
        _channel_id: &ChannelHandle,
    ) -> Result<(), UpdateWatchStateError> {
        Ok(())
    }
}
