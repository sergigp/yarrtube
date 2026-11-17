use crate::domain::event::DomainEvent;
use crate::domain::services::{
    VideoOwnerFinder, VideoOwnerFinderApi, VideoUpdater, VideoUpdaterApi,
};
use crate::domain::video::{DiscardVideoError, Video, VideoDiscarded, VideoId};
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Discards a video for good — see the `video-discard` capability.
#[derive(Clone)]
pub struct VideoDiscarder {
    video_repository: Arc<dyn VideoRepository>,
    video_updater: VideoUpdater,
    video_owner_finder: Arc<VideoOwnerFinder>,
    task_repository: Arc<dyn TaskRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
}

impl VideoDiscarder {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        task_repository: Arc<dyn TaskRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_updater: VideoUpdater::new(video_repository.clone()),
            video_repository,
            video_owner_finder,
            task_repository,
            event_publisher,
            clock,
        }
    }
}

pub trait VideoDiscarderApi: Send + Sync {
    /// The video's download Discarded for good, its pending tasks deleted,
    /// and `VideoDiscarded` published when it had a recorded file. A video
    /// already Discarded is left as it is; one In Progress or Unavailable
    /// can't be discarded.
    fn discard(&self, video_id: &VideoId) -> Result<(), DiscardVideoError>;
}

impl VideoDiscarderApi for VideoDiscarder {
    fn discard(&self, video_id: &VideoId) -> Result<(), DiscardVideoError> {
        let before = self.find_video(video_id)?;
        let now = self.clock.now();
        self.video_updater
            .update(video_id, |video| video.discard(now))
            .map_err(DiscardVideoError::Repository)?;
        self.delete_pending_tasks(video_id)?;
        before.map_or(Ok(()), |video| self.publish_discarded(&video))
    }
}

impl VideoDiscarder {
    fn find_video(&self, video_id: &VideoId) -> Result<Option<Video>, DiscardVideoError> {
        self.video_repository
            .find(video_id)
            .map_err(DiscardVideoError::Repository)
    }

    /// Every task of the video still waiting in the queue.
    fn delete_pending_tasks(&self, video_id: &VideoId) -> Result<(), DiscardVideoError> {
        self.task_repository
            .delete_pending_for_video(video_id)
            .map_err(DiscardVideoError::Repository)
    }

    /// `VideoDiscarded` for `video` as read before the discard, when it had a
    /// recorded file and its owner (where its files live) is found.
    fn publish_discarded(&self, video: &Video) -> Result<(), DiscardVideoError> {
        let Some(event) = self.discarded_event(video)? else {
            return Ok(());
        };
        self.event_publisher
            .publish(&DomainEvent::VideoDiscarded(event))
            .map_err(DiscardVideoError::Repository)
    }

    fn discarded_event(&self, video: &Video) -> Result<Option<VideoDiscarded>, DiscardVideoError> {
        let Some(filename) = video.download.filename() else {
            return Ok(None);
        };
        let owner = self
            .video_owner_finder
            .find(video)
            .map_err(DiscardVideoError::Repository)?;
        Ok(owner.map(|owner| VideoDiscarded {
            video_id: video.id.as_str().to_string(),
            output_dir: owner.target.output_dir.to_string_lossy().to_string(),
            filename: filename.to_string(),
            thumbnail_filename: video.thumbnail.filename.clone(),
            subtitle_filename: video.subtitles.filename.clone(),
        }))
    }
}
