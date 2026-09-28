use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::video::{
    ListVideosError, RecentVideo, Video, VideoSource, VideoStatus, VideoView,
};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Reads videos, listed by playlist or by channel.
#[derive(Clone)]
pub struct VideoSearcher {
    playlist_repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    clock: Arc<dyn Clock>,
}

impl VideoSearcher {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            playlist_repository,
            playlist_video_repository,
            channel_repository,
            channel_video_repository,
            video_repository,
            video_metadata_repository,
            clock,
        }
    }
}

pub trait VideoSearcherApi: Send + Sync {
    /// Lists every video recorded for a playlist, confirming the playlist
    /// exists first, ordered by playlist position, each with its generated
    /// metadata when it has any.
    fn list(&self, playlist_id: &PlaylistId) -> Result<Vec<VideoView>, ListVideosError>;

    /// Lists every video recorded for a channel, confirming the channel
    /// exists first, ordered by recency (most recent first), each with its
    /// generated metadata when it has any.
    fn list_for_channel(
        &self,
        channel_id: &ChannelHandle,
    ) -> Result<Vec<VideoView>, ListVideosError>;

    /// Lists downloaded videos across every tracked playlist and channel,
    /// newest sync first, truncated to `limit`. A video tracked by more than
    /// one source appears once per source.
    fn list_recent(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError>;

    /// Lists videos in progress across every tracked playlist and channel
    /// (see `Video::is_in_progress`), last played first, truncated to
    /// `limit`. A video tracked by more than one source appears once.
    fn list_continue_watching(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError>;

    /// Lists quick watches across every tracked playlist and channel (see
    /// `Video::is_quick_watch`), newest sync first, truncated to `limit`. A
    /// video tracked by more than one source appears once.
    fn list_quick_watches(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError>;
}

impl VideoSearcherApi for VideoSearcher {
    fn list(&self, playlist_id: &PlaylistId) -> Result<Vec<VideoView>, ListVideosError> {
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
            .map(|video| video.and_then(|video| self.view(video)))
            .collect::<anyhow::Result<Vec<VideoView>>>()
            .map_err(ListVideosError::Repository)
    }

    fn list_for_channel(
        &self,
        channel_id: &ChannelHandle,
    ) -> Result<Vec<VideoView>, ListVideosError> {
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
            .map(|video| video.and_then(|video| self.view(video)))
            .collect::<anyhow::Result<Vec<VideoView>>>()
            .map_err(ListVideosError::Repository)
    }

    fn list_recent(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError> {
        let mut recent = self.recent_from_playlists()?;
        recent.extend(self.recent_from_channels()?);

        recent.sort_by_key(|r| std::cmp::Reverse(r.video.created_at));
        recent.truncate(limit);
        Ok(recent)
    }

    fn list_continue_watching(&self, _limit: usize) -> Result<Vec<RecentVideo>, ListVideosError> {
        Ok(vec![])
    }

    fn list_quick_watches(&self, _limit: usize) -> Result<Vec<RecentVideo>, ListVideosError> {
        Ok(vec![])
    }
}

impl VideoSearcher {
    fn view(&self, video: Video) -> anyhow::Result<VideoView> {
        let metadata = self.video_metadata_repository.find(&video.id)?;
        Ok(VideoView { video, metadata })
    }

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
                                source: VideoSource::Playlist {
                                    id: playlist.id.clone(),
                                    name: playlist.name.clone(),
                                    path: playlist.path.clone(),
                                },
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
                                source: VideoSource::Channel {
                                    handle: channel.id.clone(),
                                    name: channel.name.clone(),
                                    path: channel.path.clone(),
                                    avatar_filename: channel.avatar_filename.clone(),
                                },
                            })
                            .collect()
                    })
            })
            .collect::<anyhow::Result<Vec<Vec<RecentVideo>>>>()
            .map(|nested| nested.into_iter().flatten().collect())
            .map_err(ListVideosError::Repository)
    }
}
