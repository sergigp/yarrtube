use crate::domain::channel::{Channel, ChannelView};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::sync::Arc;

/// Reads channels.
#[derive(Clone)]
pub struct ChannelSearcher {
    repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
}

impl ChannelSearcher {
    pub fn new(
        repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
    ) -> Self {
        Self {
            repository,
            channel_video_repository,
            video_repository,
        }
    }
}

pub trait ChannelSearcherApi: Send + Sync {
    /// Every channel with its count of `Downloaded`, unwatched videos.
    fn search_all(&self) -> anyhow::Result<Vec<ChannelView>>;
}

impl ChannelSearcherApi for ChannelSearcher {
    fn search_all(&self) -> anyhow::Result<Vec<ChannelView>> {
        Ok(self
            .repository
            .list()?
            .into_iter()
            .map(Self::channel_view)
            .collect())
    }
}

impl ChannelSearcher {
    fn channel_view(channel: Channel) -> ChannelView {
        ChannelView {
            id: channel.id,
            name: channel.name,
            path: channel.path,
            avatar_filename: channel.avatar_filename,
            unwatched_count: 0,
        }
    }
}
