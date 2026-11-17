use crate::domain::services::{VideoOwnerFinder, VideoUpdater, VideoUpdaterApi};
use crate::domain::video::{DiscardVideoError, VideoId};
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Discards a video for good — see the `video-discard` capability.
#[derive(Clone)]
#[allow(dead_code)] // walking skeleton
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
        let now = self.clock.now();
        self.video_updater
            .update(video_id, |video| video.discard(now))
            .map_err(DiscardVideoError::Repository)?;
        self.task_repository
            .delete_pending_for_video(video_id)
            .map_err(DiscardVideoError::Repository)
    }
}
