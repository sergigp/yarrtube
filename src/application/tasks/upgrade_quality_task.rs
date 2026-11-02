use crate::domain::services::{QualityUpgrader, QualityUpgraderApi};
use crate::domain::shared::Quality;
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::path::Path;
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
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let (video_id, quality, output_dir) = Task::decode_upgrade_quality_payload(payload)?;
        self.quality_upgrader.upgrade(
            &VideoRecordId::new(video_id)?,
            Quality::new(quality)?,
            Path::new(&output_dir),
        )
    }
}
