use crate::domain::category::CategoryMember;
use crate::domain::shared::Quality;
use crate::domain::video::{Video, VideoOwner, VideoTarget, resolve_output_dir};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use std::sync::Arc;

/// Finds the channel or playlist holding a stored video.
pub struct VideoOwnerFinder {
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    videos_path: String,
}

impl VideoOwnerFinder {
    pub fn new(
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            channel_video_repository,
            playlist_video_repository,
            channel_repository,
            playlist_repository,
            videos_path: videos_path.into(),
        }
    }
}

pub trait VideoOwnerFinderApi: Send + Sync {
    /// The channel holding `video`, else the playlist holding it; `None`
    /// when neither (or its source) is stored.
    fn find(&self, video: &Video) -> anyhow::Result<Option<VideoOwner>>;
}

impl VideoOwnerFinderApi for VideoOwnerFinder {
    fn find(&self, video: &Video) -> anyhow::Result<Option<VideoOwner>> {
        match self.find_channel_owner(video)? {
            Some(owner) => Ok(Some(owner)),
            None => self.find_playlist_owner(video),
        }
    }
}

impl VideoOwnerFinder {
    fn find_channel_owner(&self, video: &Video) -> anyhow::Result<Option<VideoOwner>> {
        let Some(channel_video) = self.channel_video_repository.find_by_video(&video.id)? else {
            return Ok(None);
        };
        Ok(self
            .channel_repository
            .find(&channel_video.channel_id)?
            .map(|channel| VideoOwner {
                source: CategoryMember::Channel(channel.id),
                target: self.target(channel.path.as_str(), channel.quality),
                sort_position: None,
            }))
    }

    fn find_playlist_owner(&self, video: &Video) -> anyhow::Result<Option<VideoOwner>> {
        let Some(playlist_video) = self.playlist_video_repository.find_by_video(&video.id)? else {
            return Ok(None);
        };
        Ok(self
            .playlist_repository
            .find(&playlist_video.playlist_id)?
            .map(|playlist| VideoOwner {
                source: CategoryMember::Playlist(playlist.id),
                target: self.target(playlist.path.as_str(), playlist.quality),
                sort_position: Some(playlist_video.position),
            }))
    }

    fn target(&self, path: &str, quality: Quality) -> VideoTarget {
        VideoTarget {
            output_dir: resolve_output_dir(&self.videos_path, path),
            quality,
        }
    }
}
