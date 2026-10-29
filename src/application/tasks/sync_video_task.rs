use crate::domain::services::{VideoReconciler, VideoReconcilerApi};
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Syncs one stored video: schedules whichever of its steps its stored
/// state still needs. Scheduled by `SyncVideoOnVideoEvent` when a video is
/// added to a playlist or channel and each time one is downloaded.
pub struct SyncVideoTask {
    video_reconciler: VideoReconciler,
}

impl SyncVideoTask {
    pub fn new(video_reconciler: VideoReconciler) -> Self {
        Self { video_reconciler }
    }
}

impl TaskHandler for SyncVideoTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let video_id = VideoRecordId::new(Task::decode_sync_video_payload(payload)?)?;
        self.video_reconciler.reconcile(&video_id)
    }
}
