pub mod dto;

use super::AppState;
use super::error::error_response;
use crate::domain::playlist::{
    CreatePlaylistError, DeletePlaylistError, PlaylistName, PlaylistPath,
};
use crate::domain::services::CreatePlaylistOutcome;
use crate::domain::shared::{PlaylistId, Quality};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dto::{CreatePlaylistRequest, PlaylistResponse};

pub async fn create_playlist(
    State(state): State<AppState>,
    Json(request): Json<CreatePlaylistRequest>,
) -> Response {
    let id = match PlaylistId::from_url_or_id(request.playlist) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let name = match PlaylistName::new(request.name) {
        Ok(name) => name,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let path = match request.path {
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Playlist path must not be empty".to_string(),
            );
        }
        Some(path) => match PlaylistPath::new(path) {
            Ok(path) => path,
            Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
        },
    };
    let quality = match request.quality {
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Quality must be one of \"high\", \"mid\", or \"low\" (missing)".to_string(),
            );
        }
        Some(quality) => match Quality::new(quality) {
            Ok(quality) => quality,
            Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
        },
    };

    let result =
        tokio::task::spawn_blocking(move || state.playlist_creator.create(id, name, path, quality))
            .await;

    match result {
        Ok(Ok(CreatePlaylistOutcome::Created(playlist))) => {
            (StatusCode::CREATED, Json(PlaylistResponse::from(playlist))).into_response()
        }
        Ok(Ok(CreatePlaylistOutcome::AlreadyExisted(playlist))) => {
            (StatusCode::OK, Json(PlaylistResponse::from(playlist))).into_response()
        }
        Ok(Err(e @ CreatePlaylistError::YoutubePlaylistNotFound(_))) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Ok(Err(e @ CreatePlaylistError::PathAlreadyInUse(_))) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Ok(Err(e @ CreatePlaylistError::Lookup(_))) => {
            error_response(StatusCode::BAD_GATEWAY, e.to_string())
        }
        Ok(Err(e @ CreatePlaylistError::Repository(_))) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn delete_playlist(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let id = match PlaylistId::new(id) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };

    match state.playlist_deleter.delete(id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e @ DeletePlaylistError::NotFound(_)) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Err(e @ DeletePlaylistError::Repository(_)) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    }
}

pub async fn reconcile_playlist(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let id = match PlaylistId::new(id) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };

    let result =
        tokio::task::spawn_blocking(move || state.video_reconciler.force_reconcile(id)).await;

    match result {
        Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(e)) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn list_playlists(State(state): State<AppState>) -> Response {
    match state.playlist_searcher.search_all() {
        Ok(playlists) => {
            let response: Vec<PlaylistResponse> =
                playlists.into_iter().map(PlaylistResponse::from).collect();
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::http::api_router;
    use crate::domain::event::DomainEvent;
    use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FakeChannelAvatarRepository;
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

    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;
    use tower::ServiceExt;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    /// Builds a fresh `ChannelVideoReconciler` wired to unrelated fakes —
    /// these tests don't exercise channel reconciliation, they just need
    /// `AppState` to construct.
    fn channel_video_reconciler(
        event_publisher: Arc<FakeEventPublisher>,
    ) -> crate::domain::services::ChannelVideoReconciler {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let thumbnail_fetcher = Arc::new(crate::domain::services::ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        crate::domain::services::ChannelVideoReconciler::new(
            Arc::new(FakeChannelRepository::default()),
            video_repository,
            Arc::new(FakeChannelVideoRepository::default()),
            Arc::new(FakeChannelVideosRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
            event_publisher,
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        )
    }

    fn test_router(
        repository: FakePlaylistRepository,
        youtube_exists: bool,
    ) -> (
        axum::Router,
        Arc<FakePlaylistRepository>,
        Arc<FakeEventPublisher>,
    ) {
        let (router, repository, event_publisher, _video_repository, _playlist_video_repository) =
            test_router_with_videos(repository, youtube_exists, FakeVideoRepository::default());
        (router, repository, event_publisher)
    }

    #[allow(clippy::type_complexity)]
    fn test_router_with_videos(
        repository: FakePlaylistRepository,
        youtube_exists: bool,
        video_repository: FakeVideoRepository,
    ) -> (
        axum::Router,
        Arc<FakePlaylistRepository>,
        Arc<FakeEventPublisher>,
        Arc<FakeVideoRepository>,
        Arc<FakePlaylistVideoRepository>,
    ) {
        let repository = Arc::new(repository);
        let video_repository = Arc::new(video_repository);
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let task_view_searcher = crate::domain::services::TaskViewSearcher::new(
            Arc::new(FakeTaskRepository::default()),
            repository.clone(),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::default()),
        );
        let thumbnail_fetcher = Arc::new(crate::domain::services::ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_reconciler = crate::domain::services::VideoReconciler::new(
            repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
            event_publisher.clone(),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let video_searcher = crate::domain::services::VideoSearcher::new(
            repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            video_repository.clone(),
        );
        let state = AppState {
            directory_searcher: crate::domain::services::DirectorySearcher::new(Arc::new(
                crate::infrastructure::repositories::filesystem_directory_repository::FakeDirectoryRepository::default(),
            )),
            videos_root: "/videos".to_string(),
            playlist_creator: crate::domain::services::PlaylistCreator::new(
                repository.clone(),
                Arc::new(FakeYoutubePlaylistRepository {
                    exists: youtube_exists,
                }),
                event_publisher.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            ),
            playlist_deleter: crate::domain::services::PlaylistDeleter::new(
                repository.clone(),
                video_repository.clone(),
                playlist_video_repository.clone(),
                event_publisher.clone(),
            ),
            playlist_searcher: crate::domain::services::PlaylistSearcher::new(repository.clone()),
            video_reconciler,
            video_searcher,
            task_view_searcher,
            channel_service: crate::domain::channel::ChannelService::new(
                Arc::new(FakeChannelRepository::default()),
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
                Arc::new(FakeVideoRepository::default()),
                Arc::new(FakeChannelVideoRepository::default()),
                event_publisher.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            ),
            channel_video_reconciler: channel_video_reconciler(event_publisher.clone()),
        };
        (
            axum::Router::new().nest("/api", api_router(state)),
            repository,
            event_publisher,
            video_repository,
            playlist_video_repository,
        )
    }

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    const DEFAULT_PATH: &str = "music/chill";

    fn create_request(id: &str, name: &str) -> Request<Body> {
        create_request_with(id, name, DEFAULT_PATH, "high")
    }

    fn create_request_with(id: &str, name: &str, path: &str, quality: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/playlists")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "playlist": id, "name": name, "path": path, "quality": quality })
                    .to_string(),
            ))
            .unwrap()
    }

    fn create_request_with_quality(id: &str, name: &str, quality: &str) -> Request<Body> {
        create_request_with(id, name, DEFAULT_PATH, quality)
    }

    fn create_request_with_path(id: &str, name: &str, path: &str) -> Request<Body> {
        create_request_with(id, name, path, "high")
    }

    fn create_request_missing_quality(id: &str, name: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/playlists")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "playlist": id, "name": name, "path": DEFAULT_PATH })
                    .to_string(),
            ))
            .unwrap()
    }

    fn create_request_missing_path(id: &str, name: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/playlists")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "playlist": id, "name": name, "quality": "high" }).to_string(),
            ))
            .unwrap()
    }

    #[tokio::test]
    async fn it_should_return_201_when_creating_a_new_playlist() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(create_request("PL1", "My Playlist"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = body_json(response).await;
        assert_eq!(body["id"], "PL1");
        assert_eq!(body["name"], "My Playlist");
        assert_eq!(body["path"], DEFAULT_PATH);
        assert_eq!(body["quality"], "high");
        assert_eq!(body["kind"], "youtube_linked");
        assert_eq!(
            body["created_at"],
            fixed_timestamp().to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
        );
    }

    #[tokio::test]
    async fn it_should_return_200_when_creating_a_playlist_that_already_exists() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);
        router
            .clone()
            .oneshot(create_request("PL1", "Original Name"))
            .await
            .unwrap();

        let response = router
            .oneshot(create_request("PL1", "Different Name"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["name"], "Original Name");
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_name_is_invalid() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router.oneshot(create_request("PL1", "")).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_is_missing() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(create_request_missing_path("PL1", "My Playlist"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_is_absolute() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(create_request_with_path(
                "PL1",
                "My Playlist",
                "/absolute/path",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_contains_a_parent_traversal_segment() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(create_request_with_path("PL1", "My Playlist", "a/../b"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_contains_an_empty_segment() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(create_request_with_path("PL1", "My Playlist", "a//b"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_path_is_already_used_by_another_playlist() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);
        router
            .clone()
            .oneshot(create_request_with_path("PL1", "First", "shared/path"))
            .await
            .unwrap();

        let response = router
            .oneshot(create_request_with_path("PL2", "Second", "shared/path"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_keep_the_existing_path_when_creating_a_duplicate_with_a_different_path() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);
        router
            .clone()
            .oneshot(create_request_with_path(
                "PL1",
                "My Playlist",
                "original/path",
            ))
            .await
            .unwrap();

        let response = router
            .oneshot(create_request_with_path(
                "PL1",
                "My Playlist",
                "different/path",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["path"], "original/path");
    }

    #[tokio::test]
    async fn it_should_return_400_when_quality_is_missing() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(create_request_missing_quality("PL1", "My Playlist"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_quality_is_invalid() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(create_request_with_quality("PL1", "My Playlist", "ultra"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_keep_the_existing_quality_when_creating_a_duplicate_with_a_different_quality()
     {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);
        router
            .clone()
            .oneshot(create_request_with_quality("PL1", "My Playlist", "high"))
            .await
            .unwrap();

        let response = router
            .oneshot(create_request_with_quality("PL1", "My Playlist", "low"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["quality"], "high");
    }

    #[tokio::test]
    async fn it_should_return_201_when_creating_a_playlist_from_a_youtube_url() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(create_request(
                "https://www.youtube.com/playlist?list=PL1",
                "My Playlist",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = body_json(response).await;
        assert_eq!(body["id"], "PL1");
    }

    #[tokio::test]
    async fn it_should_return_201_when_creating_a_playlist_from_a_watch_url_with_a_list_param() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(create_request(
                "https://youtube.com/watch?v=vid1&list=PL1",
                "My Playlist",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = body_json(response).await;
        assert_eq!(body["id"], "PL1");
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_url_is_not_a_recognized_youtube_url() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(create_request(
                "https://example.com/playlist?list=PL1",
                "My Playlist",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_youtube_url_is_missing_the_list_parameter() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(create_request(
                "https://www.youtube.com/watch?v=vid1",
                "My Playlist",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_playlist_value_is_empty() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(create_request("", "My Playlist"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_youtube_playlist_does_not_exist() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), false);

        let response = router
            .oneshot(create_request("PL404", "My Playlist"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_record_a_playlist_created_event_only_once_for_repeated_creation() {
        let (router, _repository, event_publisher) =
            test_router(FakePlaylistRepository::default(), true);
        router
            .clone()
            .oneshot(create_request("PL1", "First"))
            .await
            .unwrap();

        router
            .oneshot(create_request("PL1", "First Again"))
            .await
            .unwrap();

        let published = event_publisher.published.lock().unwrap();
        assert_eq!(
            *published,
            vec![DomainEvent::PlaylistCreated {
                playlist_id: "PL1".to_string()
            }]
        );
    }

    #[tokio::test]
    async fn it_should_return_204_when_deleting_an_existing_playlist() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);
        router
            .clone()
            .oneshot(create_request("PL1", "My Playlist"))
            .await
            .unwrap();

        let response = router
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/playlists/PL1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn it_should_record_a_playlist_deleted_event_on_successful_deletion() {
        let (router, _repository, event_publisher) =
            test_router(FakePlaylistRepository::default(), true);
        router
            .clone()
            .oneshot(create_request("PL1", "My Playlist"))
            .await
            .unwrap();

        router
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/playlists/PL1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let published = event_publisher.published.lock().unwrap();
        assert_eq!(
            *published,
            vec![
                DomainEvent::PlaylistCreated {
                    playlist_id: "PL1".to_string()
                },
                DomainEvent::PlaylistDeleted {
                    playlist_id: "PL1".to_string(),
                    path: DEFAULT_PATH.to_string(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn it_should_delete_every_video_record_for_a_deleted_youtube_linked_playlist() {
        use crate::domain::playlist_video::PlaylistVideo;
        use crate::domain::shared::VideoId;
        use crate::domain::video::Video;
        use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
        use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;

        let (router, _repository, _event_publisher, video_repository, playlist_video_repository) =
            test_router_with_videos(
                FakePlaylistRepository::default(),
                true,
                FakeVideoRepository::default(),
            );
        router
            .clone()
            .oneshot(create_request("PL1", "My Playlist"))
            .await
            .unwrap();
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());
        video_repository.save(&video).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                video.id.clone(),
                fixed_timestamp(),
            ))
            .unwrap();

        let response = router
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/playlists/PL1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(video_repository.videos.lock().unwrap().is_empty());
        assert!(
            playlist_video_repository
                .list_for_playlist(&PlaylistId::new("PL1").unwrap())
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_deleting_a_missing_playlist() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/playlists/PL404")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_an_empty_array_when_no_playlists_exist() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/playlists")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body, serde_json::json!([]));
    }

    #[tokio::test]
    async fn it_should_return_all_created_playlists() {
        let (router, _repository, _event_publisher) =
            test_router(FakePlaylistRepository::default(), true);
        router
            .clone()
            .oneshot(create_request_with_path("PL1", "First", "music/first"))
            .await
            .unwrap();
        router
            .clone()
            .oneshot(create_request_with_path("PL2", "Second", "music/second"))
            .await
            .unwrap();

        let response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/playlists")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let playlists = body.as_array().unwrap();
        assert_eq!(playlists.len(), 2);
        for playlist in playlists {
            assert_eq!(playlist["quality"], "high");
            assert_eq!(playlist["kind"], "youtube_linked");
        }
    }

    #[tokio::test]
    async fn it_should_delete_the_playlist_output_directory_from_disk_when_the_playlist_is_deleted()
    {
        use crate::application::subscribers::delete_playlist_files_on_playlist_deleted::DeletePlaylistFilesOnPlaylistDeleted;
        use crate::application::tasks::delete_playlist_files_task::DeletePlaylistFilesTask;
        use crate::domain::shared::VideoId;
        use crate::domain::video::Video;
        use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
        use crate::infrastructure::repositories::filesystem_video_file_repository::FilesystemVideoFileRepository;
        use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
        use crate::infrastructure::repositories::task_handler::TaskHandler;
        use crate::infrastructure::shared::ytdlp::test_support::unique_temp_dir;

        let videos_root = unique_temp_dir("cascade-delete-playlist-e2e");
        let output_dir = videos_root.join("music/chill");
        std::fs::create_dir_all(&output_dir).unwrap();
        std::fs::write(output_dir.join("My Video.mp4"), b"fake video bytes").unwrap();

        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        video_repository
            .save(
                &Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp())
                    .start_download(fixed_timestamp())
                    .mark_downloaded(Quality::High, "My Video.mp4", None, None, fixed_timestamp()),
            )
            .unwrap();
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let video_file_deleter = crate::domain::services::VideoFileDeleter::new(
            Arc::new(FilesystemVideoFileRepository),
            videos_root.to_str().unwrap(),
        );
        let thumbnail_fetcher = Arc::new(crate::domain::services::ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_reconciler = crate::domain::services::VideoReconciler::new(
            playlist_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
            event_publisher.clone(),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FilesystemVideoFileRepository),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            videos_root.to_str().unwrap(),
        );
        let video_searcher = crate::domain::services::VideoSearcher::new(
            playlist_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            video_repository.clone(),
        );
        let playlist_creator = crate::domain::services::PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let playlist_deleter = crate::domain::services::PlaylistDeleter::new(
            playlist_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            event_publisher.clone(),
        );
        let task_view_searcher = crate::domain::services::TaskViewSearcher::new(
            Arc::new(FakeTaskRepository::default()),
            playlist_repository.clone(),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::default()),
        );
        let playlist_searcher = crate::domain::services::PlaylistSearcher::new(playlist_repository);
        let state = AppState {
            directory_searcher: crate::domain::services::DirectorySearcher::new(Arc::new(
                crate::infrastructure::repositories::filesystem_directory_repository::FakeDirectoryRepository::default(),
            )),
            videos_root: "/videos".to_string(),
            playlist_creator,
            playlist_deleter,
            playlist_searcher,
            video_reconciler,
            video_searcher,
            task_view_searcher,
            channel_service: crate::domain::channel::ChannelService::new(
                Arc::new(FakeChannelRepository::default()),
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
                Arc::new(FakeVideoRepository::default()),
                Arc::new(FakeChannelVideoRepository::default()),
                event_publisher.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            ),
            channel_video_reconciler: channel_video_reconciler(event_publisher.clone()),
        };
        let router = axum::Router::new().nest("/api", api_router(state));

        router
            .clone()
            .oneshot(create_request_with_path(
                "PL1",
                "My Playlist",
                "music/chill",
            ))
            .await
            .unwrap();
        router
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/playlists/PL1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let playlist_deleted_payload = event_publisher
            .published
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .payload()
            .to_string();

        let subscriber_task_repository = Arc::new(FakeTaskRepository::default());
        DeletePlaylistFilesOnPlaylistDeleted::new(
            subscriber_task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        )
        .handle(&playlist_deleted_payload)
        .unwrap();
        let scheduled = subscriber_task_repository.scheduled.lock().unwrap();
        let (task, _run_at) = scheduled[0].clone();
        drop(scheduled);

        DeletePlaylistFilesTask::new(video_file_deleter)
            .handle(&task.payload().to_string(), false)
            .unwrap();

        assert!(!output_dir.exists());

        std::fs::remove_dir_all(&videos_root).ok();
    }

    fn reconcile_request(id: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(format!("/api/playlists/{id}/reconcile"))
            .body(Body::empty())
            .unwrap()
    }

    fn test_router_for_reconcile(
        repository: FakePlaylistRepository,
        playlist_items: FakeYoutubePlaylistItemsRepository,
        video_file_repository: FakeVideoFileRepository,
    ) -> (
        axum::Router,
        Arc<FakeVideoRepository>,
        Arc<FakeTaskRepository>,
    ) {
        let repository = Arc::new(repository);
        let video_repository = Arc::new(FakeVideoRepository::default());
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let task_repository = Arc::new(FakeTaskRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let task_view_searcher = crate::domain::services::TaskViewSearcher::new(
            task_repository.clone(),
            repository.clone(),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::default()),
        );
        let thumbnail_fetcher = Arc::new(crate::domain::services::ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_reconciler = crate::domain::services::VideoReconciler::new(
            repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(playlist_items),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
            event_publisher.clone(),
            task_repository.clone(),
            Arc::new(video_file_repository),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let video_searcher = crate::domain::services::VideoSearcher::new(
            repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            video_repository.clone(),
        );
        let state = AppState {
            directory_searcher: crate::domain::services::DirectorySearcher::new(Arc::new(
                crate::infrastructure::repositories::filesystem_directory_repository::FakeDirectoryRepository::default(),
            )),
            videos_root: "/videos".to_string(),
            playlist_creator: crate::domain::services::PlaylistCreator::new(
                repository.clone(),
                Arc::new(FakeYoutubePlaylistRepository { exists: true }),
                event_publisher.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            ),
            playlist_deleter: crate::domain::services::PlaylistDeleter::new(
                repository.clone(),
                video_repository.clone(),
                playlist_video_repository,
                event_publisher.clone(),
            ),
            playlist_searcher: crate::domain::services::PlaylistSearcher::new(repository),
            video_reconciler,
            video_searcher,
            task_view_searcher,
            channel_service: crate::domain::channel::ChannelService::new(
                Arc::new(FakeChannelRepository::default()),
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
                Arc::new(FakeVideoRepository::default()),
                Arc::new(FakeChannelVideoRepository::default()),
                event_publisher.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            ),
            channel_video_reconciler: channel_video_reconciler(event_publisher),
        };
        (
            axum::Router::new().nest("/api", api_router(state)),
            video_repository,
            task_repository,
        )
    }

    #[tokio::test]
    async fn it_should_return_204_and_apply_membership_changes_when_reconciling_a_youtube_linked_playlist()
     {
        use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
        use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubePlaylistItem;

        let repository = FakePlaylistRepository::default();
        repository
            .insert(&crate::domain::playlist::Playlist::create(
                PlaylistId::new("PL1").unwrap(),
                crate::domain::playlist::PlaylistName::new("My Playlist").unwrap(),
                crate::domain::playlist::PlaylistPath::new("music/chill").unwrap(),
                Quality::High,
                crate::domain::playlist::PlaylistKind::YoutubeLinked,
                fixed_timestamp(),
            ))
            .unwrap();
        let (router, video_repository, task_repository) = test_router_for_reconcile(
            repository,
            FakeYoutubePlaylistItemsRepository {
                videos: std::sync::Mutex::new(vec![YoutubePlaylistItem {
                    video_id: "vid1".to_string(),
                    title: "One".to_string(),
                    position: 0,
                }]),
            },
            FakeVideoFileRepository::default(),
        );

        let response = router.oneshot(reconcile_request("PL1")).await.unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].youtube_id.as_str(), "vid1");
        drop(stored);
        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn it_should_not_schedule_any_task_when_reconciling_repeatedly_on_demand() {
        use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;

        let repository = FakePlaylistRepository::default();
        repository
            .insert(&crate::domain::playlist::Playlist::create(
                PlaylistId::new("PL1").unwrap(),
                crate::domain::playlist::PlaylistName::new("My Playlist").unwrap(),
                crate::domain::playlist::PlaylistPath::new("music/chill").unwrap(),
                Quality::High,
                crate::domain::playlist::PlaylistKind::YoutubeLinked,
                fixed_timestamp(),
            ))
            .unwrap();
        let (router, _video_repository, task_repository) = test_router_for_reconcile(
            repository,
            FakeYoutubePlaylistItemsRepository::default(),
            FakeVideoFileRepository::default(),
        );

        router
            .clone()
            .oneshot(reconcile_request("PL1"))
            .await
            .unwrap();
        router
            .clone()
            .oneshot(reconcile_request("PL1"))
            .await
            .unwrap();
        router.oneshot(reconcile_request("PL1")).await.unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn it_should_leave_an_existing_pending_reconcile_task_untouched() {
        use crate::domain::task::Task;
        use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
        use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;

        let repository = FakePlaylistRepository::default();
        repository
            .insert(&crate::domain::playlist::Playlist::create(
                PlaylistId::new("PL1").unwrap(),
                crate::domain::playlist::PlaylistName::new("My Playlist").unwrap(),
                crate::domain::playlist::PlaylistPath::new("music/chill").unwrap(),
                Quality::High,
                crate::domain::playlist::PlaylistKind::YoutubeLinked,
                fixed_timestamp(),
            ))
            .unwrap();
        let (router, _video_repository, task_repository) = test_router_for_reconcile(
            repository,
            FakeYoutubePlaylistItemsRepository::default(),
            FakeVideoFileRepository::default(),
        );
        let existing_task = Task::ReconcilePlaylist {
            playlist_id: "PL1".to_string(),
        };
        let existing_run_at = fixed_timestamp() + chrono::Duration::seconds(1800);
        task_repository
            .schedule(&existing_task, existing_run_at)
            .unwrap();

        router.oneshot(reconcile_request("PL1")).await.unwrap();

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert_eq!(*scheduled, vec![(existing_task, existing_run_at)]);
    }

    #[tokio::test]
    async fn it_should_return_204_and_do_nothing_when_reconciling_a_nonexistent_playlist() {
        let (router, video_repository, task_repository) = test_router_for_reconcile(
            FakePlaylistRepository::default(),
            FakeYoutubePlaylistItemsRepository::default(),
            FakeVideoFileRepository::default(),
        );

        let response = router.oneshot(reconcile_request("PL404")).await.unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(video_repository.videos.lock().unwrap().is_empty());
        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn it_should_return_400_when_reconciling_with_an_invalid_playlist_id() {
        let (router, _video_repository, _task_repository) = test_router_for_reconcile(
            FakePlaylistRepository::default(),
            FakeYoutubePlaylistItemsRepository::default(),
            FakeVideoFileRepository::default(),
        );

        let response = router.oneshot(reconcile_request("%20")).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
