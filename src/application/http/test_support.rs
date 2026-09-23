//! Builds a fully wired `AppState` out of fakes, so a handler's behavior
//! tests only have to configure the one port they exercise. The older
//! handlers' tests build their own state inline; this exists for the ones
//! that would otherwise repeat the whole graph to reach a single service.

use super::{AppState, api_router};
use crate::domain::channel::ChannelService;
use crate::domain::services::{
    ChannelVideoReconciler, DirectorySearcher, PlaylistCreator, PlaylistDeleter, PlaylistSearcher,
    TaskViewSearcher, ThumbnailFetcher, VideoReconciler, VideoSearcher,
};
use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FakeChannelAvatarRepository;
use crate::infrastructure::repositories::filesystem_directory_repository::{
    DirectoryRepository, FakeDirectoryRepository,
};
use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::FakeChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::FakeChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::FakePlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::FakePlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::FakeVideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
use crate::infrastructure::repositories::youtube_channel_repository::FakeYoutubeChannelRepository;
use crate::infrastructure::repositories::youtube_channel_videos_repository::FakeChannelVideosRepository;
use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
use crate::infrastructure::repositories::youtube_playlist_repository::FakeYoutubePlaylistRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
use crate::infrastructure::shared::system_clock::FixedClock;
use axum::Router;
use chrono::{DateTime, Utc};
use std::sync::Arc;

const RECONCILE_INTERVAL_SECONDS: i64 = 3600;

fn fixed_timestamp() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
}

pub fn test_router_with_directories(
    directory_repository: FakeDirectoryRepository,
    videos_root: &str,
) -> Router {
    Router::new().nest(
        "/api",
        api_router(test_state(Arc::new(directory_repository), videos_root)),
    )
}

fn test_state(directory_repository: Arc<dyn DirectoryRepository>, videos_root: &str) -> AppState {
    let playlist_repository = Arc::new(FakePlaylistRepository::default());
    let channel_repository = Arc::new(FakeChannelRepository::default());
    let video_repository = Arc::new(FakeVideoRepository::default());
    let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
    let channel_video_repository = Arc::new(FakeChannelVideoRepository::default());
    let task_repository = Arc::new(FakeTaskRepository::default());
    let event_publisher = Arc::new(FakeEventPublisher::default());
    let clock = Arc::new(FixedClock(fixed_timestamp()));

    let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
        video_repository.clone(),
        Arc::new(FakeVideoDownloaderRepository::default()),
        clock.clone(),
    ));

    AppState {
        playlist_creator: PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher.clone(),
            clock.clone(),
        ),
        playlist_deleter: PlaylistDeleter::new(
            playlist_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            event_publisher.clone(),
        ),
        playlist_searcher: PlaylistSearcher::new(playlist_repository.clone()),
        video_reconciler: VideoReconciler::new(
            playlist_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
            event_publisher.clone(),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            thumbnail_fetcher.clone(),
            clock.clone(),
            RECONCILE_INTERVAL_SECONDS,
            videos_root,
        ),
        video_searcher: VideoSearcher::new(
            playlist_repository.clone(),
            playlist_video_repository.clone(),
            channel_repository.clone(),
            channel_video_repository.clone(),
            video_repository.clone(),
        ),
        task_view_searcher: TaskViewSearcher::new(
            task_repository.clone(),
            playlist_repository,
            channel_repository.clone(),
            video_repository.clone(),
            playlist_video_repository,
            channel_video_repository.clone(),
        ),
        channel_service: ChannelService::new(
            channel_repository.clone(),
            Arc::new(FakeYoutubeChannelRepository { resolved: None }),
            Arc::new(FakeChannelAvatarRepository::default()),
            video_repository.clone(),
            channel_video_repository.clone(),
            event_publisher.clone(),
            clock.clone(),
        ),
        channel_video_reconciler: ChannelVideoReconciler::new(
            channel_repository,
            video_repository,
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
            event_publisher,
            task_repository,
            Arc::new(FakeVideoFileRepository::default()),
            thumbnail_fetcher,
            clock,
            RECONCILE_INTERVAL_SECONDS,
            videos_root,
        ),
        directory_searcher: DirectorySearcher::new(directory_repository),
        videos_root: videos_root.to_string(),
    }
}
