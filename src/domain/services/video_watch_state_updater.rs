use crate::domain::channel::ChannelHandle;
use crate::domain::video::{
    PlaybackPosition, UpdateWatchStateError, Video, VideoDuration, VideoId, VideoStatus,
};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::collections::HashSet;
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
    /// Applies `Video::update_watch_state` to every stored copy of `youtube_id`.
    fn update(
        &self,
        youtube_id: &VideoId,
        position: PlaybackPosition,
        reported_duration: Option<VideoDuration>,
    ) -> Result<(), UpdateWatchStateError>;
    /// Marks every `Downloaded` video of the channel watched, with every
    /// stored copy of each; pending/in-flight videos are left unchanged.
    fn mark_channel_watched(&self, channel_id: &ChannelHandle)
    -> Result<(), UpdateWatchStateError>;
}

impl VideoWatchStateUpdaterApi for VideoWatchStateUpdater {
    fn update(
        &self,
        youtube_id: &VideoId,
        position: PlaybackPosition,
        reported_duration: Option<VideoDuration>,
    ) -> Result<(), UpdateWatchStateError> {
        let copies = self.find_copies(youtube_id)?;
        let now = self.clock.now();
        copies
            .into_iter()
            .map(|video| video.update_watch_state(position, reported_duration, now))
            .try_for_each(|video| self.video_repository.update(&video))
            .map_err(UpdateWatchStateError::Repository)
    }

    fn mark_channel_watched(
        &self,
        channel_id: &ChannelHandle,
    ) -> Result<(), UpdateWatchStateError> {
        self.ensure_channel_exists(channel_id)?;
        let now = self.clock.now();
        self.unwatched_downloaded_youtube_ids(channel_id)?
            .iter()
            .try_for_each(|youtube_id| self.mark_copies_watched(youtube_id, now))
    }
}

impl VideoWatchStateUpdater {
    fn ensure_channel_exists(
        &self,
        channel_id: &ChannelHandle,
    ) -> Result<(), UpdateWatchStateError> {
        self.channel_repository
            .find(channel_id)
            .map_err(UpdateWatchStateError::Repository)?
            .map(|_| ())
            .ok_or_else(|| UpdateWatchStateError::ChannelNotFound(channel_id.clone()))
    }

    /// The YouTube IDs of the channel's `Downloaded` videos not yet watched,
    /// each once.
    fn unwatched_downloaded_youtube_ids(
        &self,
        channel_id: &ChannelHandle,
    ) -> Result<HashSet<VideoId>, UpdateWatchStateError> {
        Ok(self
            .channel_videos(channel_id)?
            .into_iter()
            .filter(|video| video.status == VideoStatus::Downloaded && !video.is_watched())
            .map(|video| video.youtube_id)
            .collect())
    }

    fn channel_videos(
        &self,
        channel_id: &ChannelHandle,
    ) -> Result<Vec<Video>, UpdateWatchStateError> {
        self.channel_video_repository
            .list_for_channel(channel_id)
            .and_then(|channel_videos| {
                channel_videos
                    .iter()
                    .filter_map(|channel_video| {
                        self.video_repository
                            .find(&channel_video.video_id)
                            .transpose()
                    })
                    .collect()
            })
            .map_err(UpdateWatchStateError::Repository)
    }

    fn mark_copies_watched(
        &self,
        youtube_id: &VideoId,
        now: DateTime<Utc>,
    ) -> Result<(), UpdateWatchStateError> {
        self.video_repository
            .find_by_youtube_id(youtube_id)
            .map_err(UpdateWatchStateError::Repository)?
            .into_iter()
            .map(|video| video.mark_watched(now))
            .try_for_each(|video| self.video_repository.update(&video))
            .map_err(UpdateWatchStateError::Repository)
    }

    fn find_copies(&self, youtube_id: &VideoId) -> Result<Vec<Video>, UpdateWatchStateError> {
        Some(
            self.video_repository
                .find_by_youtube_id(youtube_id)
                .map_err(UpdateWatchStateError::Repository)?,
        )
        .filter(|copies| !copies.is_empty())
        .ok_or_else(|| UpdateWatchStateError::VideoNotFound(youtube_id.clone()))
    }
}
