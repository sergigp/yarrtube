use crate::domain::services::{VideoUpdater, VideoUpdaterApi};
use crate::domain::task::{Task, TaskStatus};
use crate::domain::video::{RetryVideoError, Video, VideoId, VideoSyncStep};
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
        let video = self.retry_steps(video_id, now)?;
        self.delete_failed_verdicts(video_id)?;
        self.bring_forward_queued_steps(&video, now)?;
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

    /// Each step task of the video waiting for a later retry runs now.
    fn bring_forward_queued_steps(
        &self,
        video: &Video,
        now: DateTime<Utc>,
    ) -> Result<(), RetryVideoError> {
        VideoSyncStep::ALL
            .iter()
            .try_for_each(|step| self.bring_forward(&step.task_for(video), &video.id, now))
            .map_err(RetryVideoError::Repository)
    }

    /// `task` of the video, when pending, runs now.
    fn bring_forward(
        &self,
        task: &Task,
        video_id: &VideoId,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        match self
            .task_repository
            .find_non_completed_for_video(task.task_type(), video_id)?
        {
            Some(queued) if queued.status == TaskStatus::Pending => {
                self.task_repository.update(&queued.bring_forward(now, now))
            }
            _ => Ok(()),
        }
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
