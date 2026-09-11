pub mod delete_video_file_task;
pub mod download_video_task;
pub mod reconcile_playlist_task;

use crate::domain::video::VideoService;
use crate::infrastructure::repositories::task_executor::HandlerRegistry;
use delete_video_file_task::DeleteVideoFileTask;
use download_video_task::DownloadVideoTask;
use reconcile_playlist_task::ReconcilePlaylistTask;
use std::collections::HashMap;
use std::sync::Arc;

/// Maps each task type to the single handler that runs it, handed to the
/// task executor at composition time.
pub fn registry(video_service: VideoService) -> HandlerRegistry {
    let mut registry: HandlerRegistry = HashMap::new();
    registry.insert(
        "reconcile_playlist".to_string(),
        Arc::new(ReconcilePlaylistTask::new(video_service.clone())),
    );
    registry.insert(
        "download_video".to_string(),
        Arc::new(DownloadVideoTask::new(video_service.clone())),
    );
    registry.insert(
        "delete_video_file".to_string(),
        Arc::new(DeleteVideoFileTask::new(video_service)),
    );
    registry
}
