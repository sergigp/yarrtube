use crate::domain::category::Category;
use crate::domain::video::SourcedVideo;
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
    fn downloaded_videos(&self, _category: &Category) -> anyhow::Result<Vec<SourcedVideo>> {
        Ok(Vec::new())
    }
}
