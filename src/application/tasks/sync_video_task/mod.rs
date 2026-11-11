use crate::domain::services::{VideoReconciler, VideoReconcilerApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};

/// Syncs one stored video: brings it to its desired state and schedules
/// whichever of its steps it still needs (`VideoReconciler`). Scheduled by
/// the `SyncVideoOn*` subscribers when a video is added to a playlist or
/// channel and each time one is downloaded.
pub struct SyncVideoTask {
    video_reconciler: VideoReconciler,
}

impl SyncVideoTask {
    pub fn new(video_reconciler: VideoReconciler) -> Self {
        Self { video_reconciler }
    }
}

impl TaskHandler for SyncVideoTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::SyncVideo { video_id } = task else {
            return Err(unexpected_task(task));
        };
        let video_id = VideoId::new(video_id.clone())?;
        self.video_reconciler
            .reconcile(&video_id)
            .map(|()| TaskOutcome::Done)
    }
}

#[cfg(test)]
mod tests;
