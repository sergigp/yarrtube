use crate::domain::channel::ChannelHandle;
use crate::domain::shared::PlaylistId;
use crate::domain::video::{ListVideosError, Video};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::sync::Arc;

/// Reads videos, listed by playlist or by channel.
#[derive(Clone)]
pub struct VideoSearcher {
    playlist_repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
}

impl VideoSearcher {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
    ) -> Self {
        Self {
            playlist_repository,
            playlist_video_repository,
            channel_repository,
            channel_video_repository,
            video_repository,
        }
    }

    /// Lists every video recorded for a playlist, confirming the playlist
    /// exists first, ordered by playlist position.
    pub fn list(&self, playlist_id: &PlaylistId) -> Result<Vec<Video>, ListVideosError> {
        self.playlist_repository
            .find(playlist_id)
            .map_err(ListVideosError::Repository)?
            .ok_or_else(|| ListVideosError::PlaylistNotFound(playlist_id.clone()))?;

        let playlist_videos = self
            .playlist_video_repository
            .list_for_playlist(playlist_id)
            .map_err(ListVideosError::Repository)?;
        playlist_videos
            .iter()
            .filter_map(|pv| self.video_repository.find(&pv.video_id).transpose())
            .collect::<anyhow::Result<Vec<Video>>>()
            .map_err(ListVideosError::Repository)
    }

    /// Lists every video recorded for a channel, confirming the channel
    /// exists first, ordered by recency (most recent first).
    pub fn list_for_channel(
        &self,
        channel_id: &ChannelHandle,
    ) -> Result<Vec<Video>, ListVideosError> {
        self.channel_repository
            .find(channel_id)
            .map_err(ListVideosError::Repository)?
            .ok_or_else(|| ListVideosError::ChannelNotFound(channel_id.clone()))?;

        let channel_videos = self
            .channel_video_repository
            .list_for_channel(channel_id)
            .map_err(ListVideosError::Repository)?;
        channel_videos
            .iter()
            .filter_map(|cv| self.video_repository.find(&cv.video_id).transpose())
            .collect::<anyhow::Result<Vec<Video>>>()
            .map_err(ListVideosError::Repository)
    }
}
