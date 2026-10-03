pub mod delete_channel_files_task;
pub mod delete_playlist_files_task;
pub mod delete_video_file_task;
pub mod download_video_task;
pub mod fetch_thumbnail_task;
#[cfg(test)]
pub(crate) mod log_capture;
pub mod reconcile_channel_task;
pub mod reconcile_playlist_task;
pub mod reconcile_plex_collections_task;
pub mod update_ytdlp_task;

use crate::domain::services::{
    ChannelVideoReconciler, PlaylistVideoReconciler, ThumbnailFetcher, VideoDownloader,
    VideoFileDeleter,
};
use crate::infrastructure::client::ytdlp_updater::YtdlpUpdater;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_executor::HandlerRegistry;
use crate::infrastructure::shared::system_clock::Clock;
use delete_channel_files_task::DeleteChannelFilesTask;
use delete_playlist_files_task::DeletePlaylistFilesTask;
use delete_video_file_task::DeleteVideoFileTask;
use download_video_task::DownloadVideoTask;
use fetch_thumbnail_task::FetchThumbnailTask;
use reconcile_channel_task::ReconcileChannelTask;
use reconcile_playlist_task::ReconcilePlaylistTask;
use reconcile_plex_collections_task::ReconcilePlexCollectionsTask;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use update_ytdlp_task::UpdateYtdlpTask;

/// Maps each task type to the single handler that runs it, handed to the
/// task executor at composition time.
#[allow(clippy::too_many_arguments)]
pub fn registry(
    playlist_video_reconciler: PlaylistVideoReconciler,
    channel_video_reconciler: ChannelVideoReconciler,
    video_downloader: VideoDownloader,
    thumbnail_fetcher: Arc<ThumbnailFetcher>,
    video_repository: Arc<dyn VideoRepository>,
    video_file_deleter: VideoFileDeleter,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    ytdlp_updater: Arc<dyn YtdlpUpdater>,
    ytdlp_path: PathBuf,
    reconcile_plex_collections: Option<ReconcilePlexCollectionsTask>,
) -> HandlerRegistry {
    let mut registry: HandlerRegistry = HashMap::new();
    registry.insert(
        "reconcile_playlist".to_string(),
        Arc::new(ReconcilePlaylistTask::new(playlist_video_reconciler)),
    );
    registry.insert(
        "reconcile_channel".to_string(),
        Arc::new(ReconcileChannelTask::new(channel_video_reconciler)),
    );
    registry.insert(
        "download_video".to_string(),
        Arc::new(DownloadVideoTask::new(video_downloader)),
    );
    registry.insert(
        "fetch_thumbnail".to_string(),
        Arc::new(FetchThumbnailTask::new(video_repository, thumbnail_fetcher)),
    );
    registry.insert(
        "delete_video_file".to_string(),
        Arc::new(DeleteVideoFileTask::new(video_file_deleter.clone())),
    );
    registry.insert(
        "delete_playlist_files".to_string(),
        Arc::new(DeletePlaylistFilesTask::new(video_file_deleter.clone())),
    );
    registry.insert(
        "delete_channel_files".to_string(),
        Arc::new(DeleteChannelFilesTask::new(video_file_deleter)),
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
    if let Some(task) = reconcile_plex_collections {
        registry.insert("reconcile_plex_collections".to_string(), Arc::new(task));
    }
    registry
}
