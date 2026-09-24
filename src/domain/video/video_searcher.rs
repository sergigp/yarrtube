use crate::domain::channel::ChannelHandle;
use crate::domain::shared::PlaylistId;
use crate::domain::video::{ListVideosError, RecentVideo, Video, VideoSource, VideoStatus};
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
}

pub trait VideoSearcherApi: Send + Sync {
    /// Lists every video recorded for a playlist, confirming the playlist
    /// exists first, ordered by playlist position.
    fn list(&self, playlist_id: &PlaylistId) -> Result<Vec<Video>, ListVideosError>;

    /// Lists every video recorded for a channel, confirming the channel
    /// exists first, ordered by recency (most recent first).
    fn list_for_channel(&self, channel_id: &ChannelHandle) -> Result<Vec<Video>, ListVideosError>;

    /// Lists downloaded videos across every tracked playlist and channel,
    /// newest sync first, truncated to `limit`. A video tracked by more than
    /// one source appears once per source.
    fn list_recent(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError>;
}

impl VideoSearcherApi for VideoSearcher {
    fn list(&self, playlist_id: &PlaylistId) -> Result<Vec<Video>, ListVideosError> {
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

    fn list_for_channel(&self, channel_id: &ChannelHandle) -> Result<Vec<Video>, ListVideosError> {
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

    fn list_recent(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError> {
        let mut recent = self.recent_from_playlists()?;
        recent.extend(self.recent_from_channels()?);

        recent.sort_by_key(|r| std::cmp::Reverse(r.video.created_at));
        recent.truncate(limit);
        Ok(recent)
    }
}

impl VideoSearcher {
    fn recent_from_playlists(&self) -> Result<Vec<RecentVideo>, ListVideosError> {
        let playlists = self
            .playlist_repository
            .list()
            .map_err(ListVideosError::Repository)?;

        playlists
            .iter()
            .map(|playlist| -> anyhow::Result<Vec<RecentVideo>> {
                let playlist_videos = self
                    .playlist_video_repository
                    .list_for_playlist(&playlist.id)?;
                playlist_videos
                    .iter()
                    .filter_map(|pv| self.video_repository.find(&pv.video_id).transpose())
                    .collect::<anyhow::Result<Vec<Video>>>()
                    .map(|videos| {
                        videos
                            .into_iter()
                            .filter(|video| video.status == VideoStatus::Downloaded)
                            .map(|video| RecentVideo {
                                video,
                                source: VideoSource::Playlist(
                                    playlist.id.clone(),
                                    playlist.path.clone(),
                                ),
                            })
                            .collect()
                    })
            })
            .collect::<anyhow::Result<Vec<Vec<RecentVideo>>>>()
            .map(|nested| nested.into_iter().flatten().collect())
            .map_err(ListVideosError::Repository)
    }

    fn recent_from_channels(&self) -> Result<Vec<RecentVideo>, ListVideosError> {
        let channels = self
            .channel_repository
            .list()
            .map_err(ListVideosError::Repository)?;

        channels
            .iter()
            .map(|channel| -> anyhow::Result<Vec<RecentVideo>> {
                let channel_videos = self
                    .channel_video_repository
                    .list_for_channel(&channel.id)?;
                channel_videos
                    .iter()
                    .filter_map(|cv| self.video_repository.find(&cv.video_id).transpose())
                    .collect::<anyhow::Result<Vec<Video>>>()
                    .map(|videos| {
                        videos
                            .into_iter()
                            .filter(|video| video.status == VideoStatus::Downloaded)
                            .map(|video| RecentVideo {
                                video,
                                source: VideoSource::Channel(
                                    channel.id.clone(),
                                    channel.path.clone(),
                                    channel.avatar_filename.clone(),
                                ),
                            })
                            .collect()
                    })
            })
            .collect::<anyhow::Result<Vec<Vec<RecentVideo>>>>()
            .map(|nested| nested.into_iter().flatten().collect())
            .map_err(ListVideosError::Repository)
    }
}
