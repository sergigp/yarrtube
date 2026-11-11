use crate::domain::services::{QualityUpgrader, QualityUpgraderApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};
use std::sync::Arc;

/// Checks one young downloaded video's quality and upgrades it in place when
/// YouTube now offers better, scheduled by the video sync (`SyncVideoTask`
/// via `VideoReconciler`) while its quality is unsettled and due.
/// Best-effort: always succeeds, so the queue never retries it.
pub struct UpgradeQualityTask {
    quality_upgrader: Arc<QualityUpgrader>,
}

impl UpgradeQualityTask {
    pub fn new(quality_upgrader: Arc<QualityUpgrader>) -> Self {
        Self { quality_upgrader }
    }
}

impl TaskHandler for UpgradeQualityTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::UpgradeQuality { video_id } = task else {
            return Err(unexpected_task(task));
        };
        self.quality_upgrader
            .upgrade(&VideoId::new(video_id.clone())?)
            .map(TaskOutcome::from)
    }
}

#[cfg(test)]
mod tests;
