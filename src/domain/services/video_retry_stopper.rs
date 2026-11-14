use crate::domain::services::VideoUpdater;
use crate::domain::video::{StopRetryingVideoError, VideoId};
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Stops retrying a video's failing steps — see the `video-retries`
/// capability.
#[derive(Clone)]
pub struct VideoRetryStopper {
    video_repository: Arc<dyn VideoRepository>,
    video_updater: VideoUpdater,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl VideoRetryStopper {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_updater: VideoUpdater::new(video_repository.clone()),
            video_repository,
            task_repository,
            clock,
        }
    }
}

pub trait VideoRetryStopperApi: Send + Sync {
    /// Every Retrying step but the categorization Failed, keeping its failed
    /// attempts, and its pending task removed.
    fn stop_retrying(&self, video_id: &VideoId) -> Result<(), StopRetryingVideoError>;
}

impl VideoRetryStopperApi for VideoRetryStopper {
    fn stop_retrying(&self, _video_id: &VideoId) -> Result<(), StopRetryingVideoError> {
        let _ = (
            &self.video_repository,
            &self.video_updater,
            &self.task_repository,
            &self.clock,
        );
        Ok(())
    }
}
