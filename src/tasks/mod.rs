pub mod delete_playlist_files_task;
pub mod delete_video_file_task;
pub mod download_video_task;
pub mod reconcile_playlist_task;
pub mod update_ytdlp_task;

use crate::domain::video::VideoService;
use crate::infrastructure::client::ytdlp_updater::YtdlpUpdater;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::task_executor::HandlerRegistry;
use crate::infrastructure::shared::system_clock::Clock;
use delete_playlist_files_task::DeletePlaylistFilesTask;
use delete_video_file_task::DeleteVideoFileTask;
use download_video_task::DownloadVideoTask;
use reconcile_playlist_task::ReconcilePlaylistTask;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use update_ytdlp_task::UpdateYtdlpTask;

/// Maps each task type to the single handler that runs it, handed to the
/// task executor at composition time.
pub fn registry(
    video_service: VideoService,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    ytdlp_updater: Arc<dyn YtdlpUpdater>,
    ytdlp_path: PathBuf,
) -> HandlerRegistry {
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
        Arc::new(DeleteVideoFileTask::new(video_service.clone())),
    );
    registry.insert(
        "delete_playlist_files".to_string(),
        Arc::new(DeletePlaylistFilesTask::new(video_service)),
    );
    registry.insert(
        "update_ytdlp".to_string(),
        Arc::new(UpdateYtdlpTask::new(
            ytdlp_updater,
            ytdlp_path,
            task_repository,
            clock,
        )),
    );
    registry
}
