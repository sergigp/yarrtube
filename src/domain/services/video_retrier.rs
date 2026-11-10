use crate::domain::services::{VideoUpdater, VideoUpdaterApi};
use crate::domain::task::Task;
use crate::domain::video::{RetryVideoError, Video, VideoId};
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::VideoCategoryVerdictRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
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
    fn retry(&self, video_id: &VideoId) -> Result<(), RetryVideoError> {
        let now = self.clock.now();
        self.retry_steps(video_id, now)?;
        self.delete_failed_verdicts(video_id)?;
        self.schedule_sync(video_id, now)
    }
}

impl VideoRetrier {
    /// Every Retrying or Failed step of the video back to Pending.
    fn retry_steps(
        &self,
        video_id: &VideoId,
        now: DateTime<Utc>,
    ) -> Result<Video, RetryVideoError> {
        self.video_updater
            .update(video_id, |video| Some(video.retry(now)))
            .map_err(RetryVideoError::Repository)?
            .ok_or_else(|| RetryVideoError::VideoNotFound(video_id.clone()))
    }

    /// The video's `failed` verdicts, so their categories are pending again.
    fn delete_failed_verdicts(&self, video_id: &VideoId) -> Result<(), RetryVideoError> {
        self.video_category_verdict_repository
            .delete_failed_for_video(video_id)
            .map_err(RetryVideoError::Repository)
    }

    fn schedule_sync(&self, video_id: &VideoId, now: DateTime<Utc>) -> Result<(), RetryVideoError> {
        let sync = Task::SyncVideo {
            video_id: video_id.as_str().to_string(),
        };
        self.task_repository
            .schedule(&sync, now)
            .map_err(RetryVideoError::Repository)
    }
}
