use crate::domain::channel::{Channel, ChannelHandle, ChannelView};
use crate::domain::video::VideoStatus;
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
        self.repository
            .list()?
            .into_iter()
            .map(|channel| self.channel_view(channel))
            .collect()
    }
}

impl ChannelSearcher {
    fn channel_view(&self, channel: Channel) -> anyhow::Result<ChannelView> {
        Ok(ChannelView {
            unwatched_count: self.count_unwatched(&channel.id)?,
            id: channel.id,
            name: channel.name,
            path: channel.path,
            avatar_filename: channel.avatar_filename,
        })
    }

    fn count_unwatched(&self, channel_id: &ChannelHandle) -> anyhow::Result<usize> {
        self.channel_video_repository
            .list_for_channel(channel_id)?
            .iter()
            .filter_map(|channel_video| {
                self.video_repository
                    .find(&channel_video.video_id)
                    .transpose()
            })
            .try_fold(0, |count, video| {
                let video = video?;
                let unwatched = video.status == VideoStatus::Downloaded && !video.is_watched();
                Ok(count + usize::from(unwatched))
            })
    }
}
