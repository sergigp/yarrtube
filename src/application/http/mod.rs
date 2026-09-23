pub mod blocking;
pub mod channels;
pub mod directories;
pub mod error;
pub mod playlists;
pub mod tasks;
pub mod validation;
pub mod videos;

use crate::domain::channel::ChannelService;
use crate::domain::services::{
    ChannelVideoReconciler, DirectorySearcher, PlaylistCreator, PlaylistDeleter, PlaylistSearcher,
    TaskViewSearcher, VideoReconciler, VideoSearcher,
};
use axum::Router;
use axum::extract::FromRef;
use axum::routing::{get, post};

/// The absolute configured videos root, carried in the adapter layer as the
/// subscribers already do, so no domain type has to know a filesystem
/// location. Reported on every directory listing.
#[derive(Clone)]
pub struct VideosRoot(pub String);

#[derive(Clone, FromRef)]
pub struct AppState {
    pub playlist_creator: PlaylistCreator,
    pub playlist_deleter: PlaylistDeleter,
    pub playlist_searcher: PlaylistSearcher,
    pub video_reconciler: VideoReconciler,
    pub video_searcher: VideoSearcher,
    pub task_view_searcher: TaskViewSearcher,
    pub channel_service: ChannelService,
    pub channel_video_reconciler: ChannelVideoReconciler,
    pub directory_searcher: DirectorySearcher,
    pub videos_root: VideosRoot,
}

pub fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/directories", get(directories::list_directories))
        .route(
            "/playlists",
            post(playlists::create_playlist).get(playlists::list_playlists),
        )
        .route(
            "/playlists/{id}",
            axum::routing::delete(playlists::delete_playlist),
        )
        .route(
            "/playlists/{id}/reconcile",
            post(playlists::reconcile_playlist),
        )
        .route(
            "/playlists/{id}/videos",
            get(videos::list_videos_for_playlist),
        )
        .route("/videos/recent", get(videos::list_recent_videos))
        .route("/tasks", get(tasks::list_tasks))
        .route(
            "/channels",
            post(channels::create_channel).get(channels::list_channels),
        )
        .route(
            "/channels/{handle}",
            axum::routing::delete(channels::delete_channel),
        )
        .route(
            "/channels/{handle}/reconcile",
            post(channels::reconcile_channel),
        )
        .route(
            "/channels/{handle}/videos",
            get(videos::list_videos_for_channel),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::services::ThumbnailFetcher;
    use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FakeChannelAvatarRepository;
    use crate::infrastructure::repositories::filesystem_directory_repository::FakeDirectoryRepository;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::SqliteChannelRepository;
    use crate::infrastructure::repositories::sqlite_channel_video_repository::SqliteChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::SqliteVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::repositories::youtube_channel_repository::FakeYoutubeChannelRepository;
    use crate::infrastructure::repositories::youtube_channel_videos_repository::FakeChannelVideosRepository;
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::repositories::youtube_playlist_repository::FakeYoutubePlaylistRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use chrono::{DateTime, Utc};
    use std::sync::Arc;
    use tower::ServiceExt;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn app_state(db: &TestDatabase) -> AppState {
        let clock = Arc::new(FixedClock(fixed_timestamp()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            clock.clone(),
        ));
        let event_publisher = Arc::new(SqliteEventPublisher::new(
            db.shared_connection(),
            clock.clone(),
        ));
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
                Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
                event_publisher.clone(),
                task_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                thumbnail_fetcher.clone(),
                clock.clone(),
                3600,
                "/videos",
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
                Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
                event_publisher,
                task_repository,
                Arc::new(FakeVideoFileRepository::default()),
                thumbnail_fetcher,
                clock,
                3600,
                "/videos",
            ),
            directory_searcher: DirectorySearcher::new(Arc::new(
                FakeDirectoryRepository::with_directories(&[("", &[])]),
            )),
            videos_root: VideosRoot("/videos".to_string()),
        }
    }

    #[tokio::test]
    async fn it_should_route_every_api_endpoint_to_a_handler() {
        let db = TestDatabase::new();
        let router = api_router(app_state(&db));
        let endpoints = [
            (Method::GET, "/directories"),
            (Method::POST, "/playlists"),
            (Method::GET, "/playlists"),
            (Method::DELETE, "/playlists/PL1"),
            (Method::POST, "/playlists/PL1/reconcile"),
            (Method::GET, "/playlists/PL1/videos"),
            (Method::GET, "/videos/recent"),
            (Method::GET, "/tasks"),
            (Method::POST, "/channels"),
            (Method::GET, "/channels"),
            (Method::DELETE, "/channels/@somechannel"),
            (Method::POST, "/channels/@somechannel/reconcile"),
            (Method::GET, "/channels/@somechannel/videos"),
        ];

        for (method, uri) in endpoints {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method.clone())
                        .uri(uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_ne!(response.status(), StatusCode::NOT_FOUND, "{method} {uri}");
            assert_ne!(
                response.status(),
                StatusCode::METHOD_NOT_ALLOWED,
                "{method} {uri}"
            );
        }
    }
}
