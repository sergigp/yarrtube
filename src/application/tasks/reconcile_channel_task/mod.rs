use crate::domain::channel::ChannelHandle;
use crate::domain::services::{ChannelVideoReconciler, ChannelVideoReconcilerApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};

/// Runs every subsequent reconcile for a channel (the first one is
/// triggered by `subscribers::reconcile_on_channel_created` instead).
pub struct ReconcileChannelTask {
    channel_video_reconciler: ChannelVideoReconciler,
}

impl ReconcileChannelTask {
    pub fn new(channel_video_reconciler: ChannelVideoReconciler) -> Self {
        Self {
            channel_video_reconciler,
        }
    }
}

impl TaskHandler for ReconcileChannelTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::ReconcileChannel { channel_id } = task else {
            return Err(unexpected_task(task));
        };
        let Ok(channel_id) = ChannelHandle::new(channel_id.clone()) else {
            return Ok(TaskOutcome::Done);
        };
        self.channel_video_reconciler
            .reconcile(channel_id)
            .map(|()| TaskOutcome::Done)
    }
}

#[cfg(test)]
mod tests;
