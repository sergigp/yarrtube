use crate::domain::playlist::PlaylistId;
use crate::domain::services::{PlaylistVideoReconciler, PlaylistVideoReconcilerApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};

/// Runs every subsequent reconcile for a playlist (the first one is
/// triggered by `subscribers::reconcile_on_playlist_created` instead).
pub struct ReconcilePlaylistTask {
    playlist_video_reconciler: PlaylistVideoReconciler,
}

impl ReconcilePlaylistTask {
    pub fn new(playlist_video_reconciler: PlaylistVideoReconciler) -> Self {
        Self {
            playlist_video_reconciler,
        }
    }
}

impl TaskHandler for ReconcilePlaylistTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::ReconcilePlaylist { playlist_id } = task else {
            return Err(unexpected_task(task));
        };
        let Ok(playlist_id) = PlaylistId::new(playlist_id.clone()) else {
            return Ok(TaskOutcome::Done);
        };
        self.playlist_video_reconciler
            .reconcile(playlist_id)
            .map(|()| TaskOutcome::Done)
    }
}

#[cfg(test)]
mod tests;
