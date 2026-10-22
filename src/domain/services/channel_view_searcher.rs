use crate::domain::channel::{Channel, ChannelHandle, ChannelView};
use crate::domain::event::ScheduledEvent;
use crate::domain::show::has_work_in_progress;
use crate::domain::task::ScheduledTask;
use crate::domain::video::{Video, VideoRecordId};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::domain_events::event_repository::EventRepository;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Reads channels as `ChannelView`s.
#[derive(Clone)]
pub struct ChannelViewSearcher {
    repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    task_repository: Arc<dyn TaskRepository>,
    event_repository: Arc<dyn EventRepository>,
    videos_root: PathBuf,
}

impl ChannelViewSearcher {
    pub fn new(
        repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        task_repository: Arc<dyn TaskRepository>,
        event_repository: Arc<dyn EventRepository>,
        videos_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            repository,
            channel_video_repository,
            video_repository,
            task_repository,
            event_repository,
            videos_root: videos_root.into(),
        }
    }
}

pub trait ChannelViewSearcherApi: Send + Sync {
    fn search_all(&self) -> anyhow::Result<Vec<ChannelView>>;
}

impl ChannelViewSearcherApi for ChannelViewSearcher {
    fn search_all(&self) -> anyhow::Result<Vec<ChannelView>> {
        let channels = self.repository.list()?;
        let videos = self.videos_by_channel()?;
        let tasks = self.task_repository.list_non_completed()?;
        let events = self.event_repository.list_eligible()?;
        Ok(channels
            .into_iter()
            .map(|channel| {
                let channel_videos = videos.get(&channel.id).map(Vec::as_slice).unwrap_or(&[]);
                self.channel_view(channel, channel_videos, &tasks, &events)
            })
            .collect())
    }
}

impl ChannelViewSearcher {
    /// Every stored video, grouped by the channel it belongs to. Channels
    /// with none are absent.
    fn videos_by_channel(&self) -> anyhow::Result<HashMap<ChannelHandle, Vec<Video>>> {
        let channel_videos = self.channel_video_repository.list()?;
        let video_ids: Vec<VideoRecordId> = channel_videos
            .iter()
            .map(|channel_video| channel_video.video_id.clone())
            .collect();
        let videos: HashMap<VideoRecordId, Video> = self
            .video_repository
            .find_many(&video_ids)?
            .into_iter()
            .map(|video| (video.id.clone(), video))
            .collect();
        Ok(channel_videos
            .into_iter()
            .filter_map(|channel_video| {
                let video = videos.get(&channel_video.video_id)?.clone();
                Some((channel_video.channel_id, video))
            })
            .fold(HashMap::new(), |mut grouped, (channel_id, video)| {
                grouped.entry(channel_id).or_default().push(video);
                grouped
            }))
    }

    fn channel_view(
        &self,
        channel: Channel,
        videos: &[Video],
        tasks: &[ScheduledTask],
        events: &[ScheduledEvent],
    ) -> ChannelView {
        let show_dir = self.videos_root.join(channel.path.as_str());
        ChannelView {
            unwatched_count: videos
                .iter()
                .filter(|video| video.is_downloaded_and_unwatched())
                .count(),
            work_in_progress: has_work_in_progress(videos, tasks, events, &show_dir),
            id: channel.id,
            name: channel.name,
            path: channel.path,
            quality: channel.quality,
            video_limit: channel.video_limit,
            reconcile_interval: channel.reconcile_interval,
            avatar_filename: channel.avatar_filename,
        }
    }
}
