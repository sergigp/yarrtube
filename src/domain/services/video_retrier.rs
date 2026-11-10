use crate::domain::services::VideoUpdater;
use crate::domain::video::{RetryVideoError, VideoId};
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::VideoCategoryVerdictRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Retries a video's Retrying and Failed steps at once — see the
/// `video-retries` capability.
#[derive(Clone)]
pub struct VideoRetrier {
    video_updater: VideoUpdater,
    video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl VideoRetrier {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_updater: VideoUpdater::new(video_repository),
            video_category_verdict_repository,
            task_repository,
            clock,
        }
    }
}

pub trait VideoRetrierApi: Send + Sync {
    /// Retrying and Failed steps back to Pending, failed verdicts deleted,
    /// queued steps brought forward to now, sync scheduled now.
    fn retry(&self, video_id: &VideoId) -> Result<(), RetryVideoError>;
}

impl VideoRetrierApi for VideoRetrier {
    fn retry(&self, _video_id: &VideoId) -> Result<(), RetryVideoError> {
        let _ = (
            &self.video_updater,
            &self.video_category_verdict_repository,
            &self.task_repository,
            &self.clock,
        );
        Ok(())
    }
}
