use crate::domain::services::{VideoUpdater, VideoUpdaterApi};
use crate::domain::task::{Task, TaskStatus};
use crate::domain::video::{StopRetryingVideoError, Video, VideoId};
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
    fn stop_retrying(&self, video_id: &VideoId) -> Result<(), StopRetryingVideoError> {
        let steps = self.find_video(video_id)?.retrying_steps();
        if steps.is_empty() {
            return Ok(());
        }
        let video = self.fail_retrying_steps(video_id)?;
        steps
            .iter()
            .try_for_each(|step| self.delete_pending_task(&step.task_for(&video), video_id))
            .map_err(StopRetryingVideoError::Repository)
    }
}

impl VideoRetryStopper {
    fn find_video(&self, video_id: &VideoId) -> Result<Video, StopRetryingVideoError> {
        self.video_repository
            .find(video_id)
            .map_err(StopRetryingVideoError::Repository)?
            .ok_or_else(|| StopRetryingVideoError::VideoNotFound(video_id.clone()))
    }

    /// Every Retrying step of the video, but the categorization, Failed.
    fn fail_retrying_steps(&self, video_id: &VideoId) -> Result<Video, StopRetryingVideoError> {
        let now = self.clock.now();
        self.video_updater
            .update(video_id, |video| Some(video.stop_retrying(now)))
            .map_err(StopRetryingVideoError::Repository)?
            .ok_or_else(|| StopRetryingVideoError::VideoNotFound(video_id.clone()))
    }

    /// `task` of the video, when pending, removed.
    fn delete_pending_task(&self, task: &Task, video_id: &VideoId) -> anyhow::Result<()> {
        match self
            .task_repository
            .find_non_completed_for_video(task.task_type(), video_id)?
        {
            Some(queued) if queued.status == TaskStatus::Pending => {
                self.task_repository.delete(queued.id)
            }
            _ => Ok(()),
        }
    }
}
