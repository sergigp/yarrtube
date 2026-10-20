use crate::domain::category::{Category, CategoryMember};
use crate::domain::channel::Channel;
use crate::domain::playlist::Playlist;
use crate::domain::video::{SourcedVideo, Video, VideoRecordId, VideoSource};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::sync::Arc;

/// Collects the downloaded videos of a category's members, shared by the
/// paths that list, count and mark a category's videos.
#[derive(Clone)]
pub struct CategoryVideoCollector {
    channel_repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
}

impl CategoryVideoCollector {
    pub fn new(
        channel_repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
    ) -> Self {
        Self {
            channel_repository,
            channel_video_repository,
            playlist_repository,
            playlist_video_repository,
            video_repository,
        }
    }
}

pub trait CategoryVideoCollectorApi: Send + Sync {
    /// Downloaded videos of the category's members that still exist, once per
    /// member, channel members first. Members already deleted are skipped.
    fn downloaded_videos(&self, category: &Category) -> anyhow::Result<Vec<SourcedVideo>>;
}

impl CategoryVideoCollectorApi for CategoryVideoCollector {
    fn downloaded_videos(&self, category: &Category) -> anyhow::Result<Vec<SourcedVideo>> {
        let from_channels = self
            .member_channels(category)?
            .iter()
            .map(|channel| self.downloaded_from_channel(channel))
            .collect::<anyhow::Result<Vec<_>>>()?;
        let from_playlists = self
            .member_playlists(category)?
            .iter()
            .map(|playlist| self.downloaded_from_playlist(playlist))
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(from_channels
            .into_iter()
            .chain(from_playlists)
            .flatten()
            .collect())
    }
}

impl CategoryVideoCollector {
    /// The tracked channels among the category's members.
    fn member_channels(&self, category: &Category) -> anyhow::Result<Vec<Channel>> {
        category
            .members
            .iter()
            .filter_map(|member| match member {
                CategoryMember::Channel(handle) => self.channel_repository.find(handle).transpose(),
                CategoryMember::Playlist(_) => None,
            })
            .collect()
    }

    /// The tracked playlists among the category's members.
    fn member_playlists(&self, category: &Category) -> anyhow::Result<Vec<Playlist>> {
        category
            .members
            .iter()
            .filter_map(|member| match member {
                CategoryMember::Playlist(id) => self.playlist_repository.find(id).transpose(),
                CategoryMember::Channel(_) => None,
            })
            .collect()
    }

    fn downloaded_from_channel(&self, channel: &Channel) -> anyhow::Result<Vec<SourcedVideo>> {
        let video_ids = self
            .channel_video_repository
            .list_for_channel(&channel.id)?
            .into_iter()
            .map(|channel_video| channel_video.video_id)
            .collect::<Vec<_>>();
        let source = VideoSource::Channel {
            handle: channel.id.clone(),
            name: channel.name.clone(),
            path: channel.path.clone(),
            avatar_filename: channel.avatar_filename.clone(),
        };
        self.downloaded(&video_ids, source)
    }

    fn downloaded_from_playlist(&self, playlist: &Playlist) -> anyhow::Result<Vec<SourcedVideo>> {
        let video_ids = self
            .playlist_video_repository
            .list_for_playlist(&playlist.id)?
            .into_iter()
            .map(|playlist_video| playlist_video.video_id)
            .collect::<Vec<_>>();
        let source = VideoSource::Playlist {
            id: playlist.id.clone(),
            name: playlist.name.clone(),
            path: playlist.path.clone(),
        };
        self.downloaded(&video_ids, source)
    }

    /// The downloaded videos of `video_ids`, each from `source`.
    fn downloaded(
        &self,
        video_ids: &[VideoRecordId],
        source: VideoSource,
    ) -> anyhow::Result<Vec<SourcedVideo>> {
        Ok(self
            .video_repository
            .find_many(video_ids)?
            .into_iter()
            .filter(Video::is_downloaded)
            .map(|video| SourcedVideo {
                video,
                source: source.clone(),
            })
            .collect())
    }
}
