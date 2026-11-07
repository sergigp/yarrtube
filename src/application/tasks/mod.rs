pub mod categorize_video_task;
pub mod delete_channel_files_task;
pub mod delete_playlist_files_task;
pub mod delete_video_file_task;
pub mod download_video_task;
pub mod fetch_chapters_task;
pub mod fetch_subtitles_task;
pub mod fetch_thumbnail_task;
#[cfg(test)]
pub(crate) mod log_capture;
pub mod reconcile_channel_task;
pub mod reconcile_playlist_task;
pub mod summarize_video_task;
pub mod sync_plex_labels_task;
pub mod sync_video_task;
pub mod update_ytdlp_task;
pub mod upgrade_quality_task;

use crate::domain::services::{
    ChannelVideoReconciler, ChapterFetcher, PlaylistVideoReconciler, PlexLabelSyncer,
    QualityUpgrader, SubtitleFetcher, ThumbnailFetcher, VideoCategorizer, VideoDownloader,
    VideoFileDeleter, VideoReconciler, VideoSummarizer,
};
use crate::infrastructure::client::ytdlp_updater::YtdlpUpdater;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_executor::HandlerRegistry;
use crate::infrastructure::shared::system_clock::Clock;
use categorize_video_task::CategorizeVideoTask;
use delete_channel_files_task::DeleteChannelFilesTask;
use delete_playlist_files_task::DeletePlaylistFilesTask;
use delete_video_file_task::DeleteVideoFileTask;
use download_video_task::DownloadVideoTask;
use fetch_chapters_task::FetchChaptersTask;
use fetch_subtitles_task::FetchSubtitlesTask;
use fetch_thumbnail_task::FetchThumbnailTask;
use reconcile_channel_task::ReconcileChannelTask;
use reconcile_playlist_task::ReconcilePlaylistTask;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use summarize_video_task::SummarizeVideoTask;
use sync_plex_labels_task::SyncPlexLabelsTask;
use sync_video_task::SyncVideoTask;
use update_ytdlp_task::UpdateYtdlpTask;
use upgrade_quality_task::UpgradeQualityTask;

/// Maps each task type to the single handler that runs it, handed to the
/// task executor at composition time.
#[allow(clippy::too_many_arguments)]
pub fn registry(
    playlist_video_reconciler: PlaylistVideoReconciler,
    channel_video_reconciler: ChannelVideoReconciler,
    video_downloader: VideoDownloader,
    thumbnail_fetcher: Arc<ThumbnailFetcher>,
    subtitle_fetcher: Arc<SubtitleFetcher>,
    chapter_fetcher: Arc<ChapterFetcher>,
    quality_upgrader: Arc<QualityUpgrader>,
    video_summarizer: Arc<VideoSummarizer>,
    video_categorizer: Arc<VideoCategorizer>,
    video_reconciler: VideoReconciler,
    video_repository: Arc<dyn VideoRepository>,
    video_file_deleter: VideoFileDeleter,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    ytdlp_updater: Arc<dyn YtdlpUpdater>,
    ytdlp_path: PathBuf,
    ytdlp_self_update_enabled: bool,
    plex_label_syncer: Option<PlexLabelSyncer>,
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
        Arc::new(FetchThumbnailTask::new(
            video_repository.clone(),
            thumbnail_fetcher,
        )),
    );
    registry.insert(
        "fetch_subtitles".to_string(),
        Arc::new(FetchSubtitlesTask::new(
            video_repository.clone(),
            subtitle_fetcher,
        )),
    );
    registry.insert(
        "fetch_chapters".to_string(),
        Arc::new(FetchChaptersTask::new(
            video_repository.clone(),
            chapter_fetcher,
        )),
    );
    registry.insert(
        "summarize_video".to_string(),
        Arc::new(SummarizeVideoTask::new(
            video_repository.clone(),
            video_summarizer,
        )),
    );
    registry.insert(
        "categorize_video".to_string(),
        Arc::new(CategorizeVideoTask::new(
            video_repository,
            video_categorizer,
        )),
    );
    registry.insert(
        "upgrade_quality".to_string(),
        Arc::new(UpgradeQualityTask::new(quality_upgrader)),
    );
    registry.insert(
        "sync_video".to_string(),
        Arc::new(SyncVideoTask::new(video_reconciler)),
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
            ytdlp_self_update_enabled,
            task_repository,
            clock,
        )),
    );
    if let Some(syncer) = plex_label_syncer {
        registry.insert(
            "sync_plex_labels".to_string(),
            Arc::new(SyncPlexLabelsTask::new(syncer)),
        );
    }
    registry
}
