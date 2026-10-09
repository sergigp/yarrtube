use crate::domain::channel::{Channel, ChannelHandle, ChannelView};
use crate::domain::channel_video::ChannelVideo;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Reads channels as `ChannelView`s.
#[derive(Clone)]
pub struct ChannelViewSearcher {
    repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
}

impl ChannelViewSearcher {
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

pub trait ChannelViewSearcherApi: Send + Sync {
    fn search_all(&self) -> anyhow::Result<Vec<ChannelView>>;
}

impl ChannelViewSearcherApi for ChannelViewSearcher {
    fn search_all(&self) -> anyhow::Result<Vec<ChannelView>> {
        let channels = self.repository.list()?;
        let counts = self.unwatched_counts()?;
        Ok(channels
            .into_iter()
            .map(|channel| Self::channel_view(channel, &counts))
            .collect())
    }
}

impl ChannelViewSearcher {
    /// Downloaded, unwatched videos per channel. Channels with none are absent.
    fn unwatched_counts(&self) -> anyhow::Result<HashMap<ChannelHandle, usize>> {
        let channel_videos = self.channel_video_repository.list()?;
        let unwatched = self.unwatched_video_ids(&channel_videos)?;
        Ok(channel_videos
            .into_iter()
            .filter(|channel_video| unwatched.contains(&channel_video.video_id))
            .fold(HashMap::new(), |mut counts, channel_video| {
                *counts.entry(channel_video.channel_id).or_insert(0) += 1;
                counts
            }))
    }

    fn unwatched_video_ids(
        &self,
        channel_videos: &[ChannelVideo],
    ) -> anyhow::Result<HashSet<VideoRecordId>> {
        let video_ids: Vec<VideoRecordId> = channel_videos
            .iter()
            .map(|channel_video| channel_video.video_id.clone())
            .collect();
        Ok(self
            .video_repository
            .find_many(&video_ids)?
            .into_iter()
            .filter(|video| video.is_downloaded_and_unwatched())
            .map(|video| video.id)
            .collect())
    }

    fn channel_view(channel: Channel, counts: &HashMap<ChannelHandle, usize>) -> ChannelView {
        ChannelView {
            unwatched_count: counts.get(&channel.id).copied().unwrap_or(0),
            id: channel.id,
            name: channel.name,
            path: channel.path,
            quality: channel.quality,
            video_limit: channel.video_limit,
            avatar_filename: channel.avatar_filename,
        }
    }
}
