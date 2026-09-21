pub mod dto;

use super::AppState;
use super::error::error_response;
use crate::domain::channel::{
    ChannelHandle, CreateChannelError, CreateChannelOutcome, DeleteChannelError, VideoLimit,
};
use crate::domain::playlist::PlaylistPath;
use crate::domain::shared::Quality;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dto::{ChannelResponse, CreateChannelRequest};

pub async fn create_channel(
    State(state): State<AppState>,
    Json(request): Json<CreateChannelRequest>,
) -> Response {
    let id = match ChannelHandle::from_url_or_handle(request.channel.unwrap_or_default()) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
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
    let video_limit = match request.video_limit {
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Video limit must be a positive integer (missing)".to_string(),
            );
        }
        Some(video_limit) => match VideoLimit::new(video_limit) {
            Ok(video_limit) => video_limit,
            Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
        },
    };
    let path = match request.path {
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Channel path must not be empty".to_string(),
            );
        }
        Some(path) => match PlaylistPath::new(path) {
            Ok(path) => path,
            Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
        },
    };

    let result = tokio::task::spawn_blocking(move || {
        state
            .channel_service
            .create_channel(id, quality, video_limit, path)
    })
    .await;

    match result {
        Ok(Ok(CreateChannelOutcome::Created(channel))) => {
            (StatusCode::CREATED, Json(ChannelResponse::from(channel))).into_response()
        }
        Ok(Ok(CreateChannelOutcome::AlreadyExisted(channel))) => {
            (StatusCode::OK, Json(ChannelResponse::from(channel))).into_response()
        }
        Ok(Err(e @ CreateChannelError::YoutubeChannelNotFound(_))) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Ok(Err(e @ CreateChannelError::Lookup(_))) => {
            error_response(StatusCode::BAD_GATEWAY, e.to_string())
        }
        Ok(Err(e @ CreateChannelError::Repository(_))) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn delete_channel(State(state): State<AppState>, Path(handle): Path<String>) -> Response {
    let id = match ChannelHandle::new(handle) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };

    match state.channel_service.delete_channel(id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e @ DeleteChannelError::NotFound(_)) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Err(e @ DeleteChannelError::Repository(_)) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    }
}

pub async fn list_channels(State(state): State<AppState>) -> Response {
    match state.channel_service.list_channels() {
        Ok(channels) => {
            let response: Vec<ChannelResponse> =
                channels.into_iter().map(ChannelResponse::from).collect();
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn reconcile_channel(
    State(state): State<AppState>,
    Path(handle): Path<String>,
) -> Response {
    let id = match ChannelHandle::new(handle) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };

    let result =
        tokio::task::spawn_blocking(move || state.channel_video_reconciler.force_reconcile(id))
            .await;

    match result {
        Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(e)) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::http::api_router;
    use crate::domain::channel::ChannelService;
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
    use crate::infrastructure::repositories::youtube_channel_repository::{
        FakeYoutubeChannelRepository, ResolvedChannel, YoutubeChannelRepository,
    };
    use crate::infrastructure::repositories::youtube_channel_videos_repository::{
        ChannelVideoListing, FakeChannelVideosRepository,
    };
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

    fn resolved_channel() -> ResolvedChannel {
        ResolvedChannel {
            youtube_channel_id: "UC123".to_string(),
            title: "Some Channel".to_string(),
            avatar_url: None,
        }
    }

    fn test_router_with(
        lookup: Arc<dyn YoutubeChannelRepository>,
        channel_videos: Vec<ChannelVideoListing>,
    ) -> (
        axum::Router,
        Arc<FakeEventPublisher>,
        Arc<FakeChannelVideoRepository>,
    ) {
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_repository = Arc::new(FakeTaskRepository::default());
        let channel_repository = Arc::new(FakeChannelRepository::default());
        let channel_video_repository = Arc::new(FakeChannelVideoRepository::default());
        let task_view_searcher = crate::domain::services::TaskViewSearcher::new(
            task_repository.clone(),
            playlist_repository.clone(),
            channel_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            channel_video_repository.clone(),
        );
        let playlist_creator = crate::domain::services::PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher.clone() as Arc<dyn crate::infrastructure::shared::domain_events::event_publisher::EventPublisher>,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let playlist_deleter = crate::domain::services::PlaylistDeleter::new(
            playlist_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            event_publisher.clone() as Arc<dyn crate::infrastructure::shared::domain_events::event_publisher::EventPublisher>,
        );
        let playlist_searcher =
            crate::domain::services::PlaylistSearcher::new(playlist_repository.clone());
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
            event_publisher.clone() as Arc<dyn crate::infrastructure::shared::domain_events::event_publisher::EventPublisher>,
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let video_searcher = crate::domain::services::VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            channel_repository.clone(),
            channel_video_repository.clone(),
            video_repository.clone(),
        );
        let channel_service = ChannelService::new(
            channel_repository.clone(),
            lookup,
            Arc::new(FakeChannelAvatarRepository::default()),
            video_repository,
            channel_video_repository.clone(),
            event_publisher.clone() as Arc<dyn crate::infrastructure::shared::domain_events::event_publisher::EventPublisher>,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let channel_video_repository_for_reconciler = Arc::new(FakeVideoRepository::default());
        let channel_thumbnail_fetcher = Arc::new(crate::domain::services::ThumbnailFetcher::new(
            channel_video_repository_for_reconciler.clone(),
            Arc::new(crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let channel_video_reconciler = crate::domain::services::ChannelVideoReconciler::new(
            channel_repository,
            channel_video_repository_for_reconciler,
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(channel_videos)),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
            event_publisher.clone() as Arc<dyn crate::infrastructure::shared::domain_events::event_publisher::EventPublisher>,
            task_repository,
            Arc::new(FakeVideoFileRepository::default()),
            channel_thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let state = AppState {
            playlist_creator,
            playlist_deleter,
            playlist_searcher,
            video_reconciler,
            video_searcher,
            task_view_searcher,
            channel_service,
            channel_video_reconciler,
        };
        (
            axum::Router::new().nest("/api", api_router(state)),
            event_publisher,
            channel_video_repository,
        )
    }

    fn test_router(resolved: Option<ResolvedChannel>) -> (axum::Router, Arc<FakeEventPublisher>) {
        let (router, events, _channel_videos) = test_router_with(
            Arc::new(FakeYoutubeChannelRepository { resolved }),
            Vec::new(),
        );
        (router, events)
    }

    struct FailingYoutubeChannelRepository;

    impl YoutubeChannelRepository for FailingYoutubeChannelRepository {
        fn resolve(
            &self,
            _handle: &crate::domain::channel::ChannelHandle,
        ) -> anyhow::Result<Option<ResolvedChannel>> {
            anyhow::bail!("YouTube API request failed")
        }
    }

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn create_request(channel: &str, quality: &str, video_limit: i64) -> Request<Body> {
        create_request_with_path(channel, quality, video_limit, "creators/somechannel")
    }

    fn create_request_with_path(
        channel: &str,
        quality: &str,
        video_limit: i64,
        path: &str,
    ) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/channels")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "channel": channel, "quality": quality, "video_limit": video_limit, "path": path })
                    .to_string(),
            ))
            .unwrap()
    }

    fn create_request_body(body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/channels")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    fn delete_request(handle: &str) -> Request<Body> {
        Request::builder()
            .method("DELETE")
            .uri(format!("/api/channels/{handle}"))
            .body(Body::empty())
            .unwrap()
    }

    fn list_request() -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri("/api/channels")
            .body(Body::empty())
            .unwrap()
    }

    fn reconcile_request(handle: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(format!("/api/channels/{handle}/reconcile"))
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn it_should_return_201_when_creating_a_new_channel_from_a_bare_handle() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request("@somechannel", "high", 10))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = body_json(response).await;
        assert_eq!(body["id"], "@somechannel");
        assert_eq!(body["name"], "Some Channel");
        assert_eq!(body["youtube_channel_id"], "UC123");
        assert_eq!(body["quality"], "high");
        assert_eq!(body["video_limit"], 10);
        assert_eq!(body["path"], "creators/somechannel");
    }

    #[tokio::test]
    async fn it_should_return_201_when_creating_a_channel_from_a_url() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request_with_path(
                "https://www.youtube.com/@somechannel",
                "high",
                10,
                "creators/somechannel",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = body_json(response).await;
        assert_eq!(body["id"], "@somechannel");
    }

    #[tokio::test]
    async fn it_should_return_200_when_creating_a_channel_that_already_exists() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));
        router
            .clone()
            .oneshot(create_request("@somechannel", "high", 10))
            .await
            .unwrap();

        let response = router
            .oneshot(create_request_with_path(
                "@somechannel",
                "low",
                5,
                "different/path",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["quality"], "high");
        assert_eq!(body["video_limit"], 10);
        assert_eq!(body["path"], "creators/somechannel");
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_channel_value_is_missing() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request_body(
                serde_json::json!({ "quality": "high", "video_limit": 10, "path": "creators/somechannel" }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_channel_value_is_empty() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request("", "high", 10))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_handle_is_missing_its_leading_at() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request("somechannel", "high", 10))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_url_is_not_a_recognized_youtube_url() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request(
                "https://example.com/@somechannel",
                "high",
                10,
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_youtube_url_is_missing_a_handle() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request(
                "https://www.youtube.com/channel/UCabcdefghijklmnopqrstuv",
                "high",
                10,
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_youtube_channel_does_not_exist() {
        let (router, _event_publisher) = test_router(None);

        let response = router
            .oneshot(create_request("@missing", "high", 10))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_502_when_the_youtube_lookup_fails() {
        let (router, _event_publisher, _channel_videos) =
            test_router_with(Arc::new(FailingYoutubeChannelRepository), Vec::new());

        let response = router
            .oneshot(create_request("@somechannel", "high", 10))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn it_should_return_400_when_quality_is_missing() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request_body(
                serde_json::json!({ "channel": "@somechannel", "video_limit": 10, "path": "creators/somechannel" }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_quality_is_invalid() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request("@somechannel", "ultra", 10))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_video_limit_is_missing() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request_body(
                serde_json::json!({ "channel": "@somechannel", "quality": "high", "path": "creators/somechannel" }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_video_limit_is_not_positive() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request("@somechannel", "high", 0))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_is_missing() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request_body(
                serde_json::json!({ "channel": "@somechannel", "quality": "high", "video_limit": 10 }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_is_absolute() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request_with_path(
                "@somechannel",
                "high",
                10,
                "/absolute/path",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_contains_a_parent_traversal_segment() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request_with_path(
                "@somechannel",
                "high",
                10,
                "a/../b",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_contains_an_empty_segment() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request_with_path("@somechannel", "high", 10, "a//b"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_record_a_channel_created_event_only_once_for_repeated_creation() {
        let (router, event_publisher) = test_router(Some(resolved_channel()));
        router
            .clone()
            .oneshot(create_request("@somechannel", "high", 10))
            .await
            .unwrap();

        router
            .oneshot(create_request("@somechannel", "low", 5))
            .await
            .unwrap();

        let published = event_publisher.published.lock().unwrap();
        assert_eq!(
            *published,
            vec![DomainEvent::ChannelCreated {
                channel_id: "@somechannel".to_string()
            }]
        );
    }

    #[tokio::test]
    async fn it_should_return_204_when_deleting_an_existing_channel() {
        let (router, event_publisher) = test_router(Some(resolved_channel()));
        router
            .clone()
            .oneshot(create_request("@somechannel", "high", 10))
            .await
            .unwrap();

        let response = router
            .oneshot(delete_request("@somechannel"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let published = event_publisher.published.lock().unwrap();
        assert_eq!(
            *published,
            vec![
                DomainEvent::ChannelCreated {
                    channel_id: "@somechannel".to_string()
                },
                DomainEvent::ChannelDeleted {
                    channel_id: "@somechannel".to_string(),
                    path: "creators/somechannel".to_string(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_deleting_a_missing_channel() {
        let (router, event_publisher) = test_router(Some(resolved_channel()));

        let response = router.oneshot(delete_request("@missing")).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(event_publisher.published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn it_should_return_an_empty_array_when_no_channels_exist() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router.oneshot(list_request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body, serde_json::json!([]));
    }

    #[tokio::test]
    async fn it_should_return_all_created_channels() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));
        router
            .clone()
            .oneshot(create_request("@somechannel", "high", 10))
            .await
            .unwrap();

        let response = router.oneshot(list_request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let channels = body.as_array().unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0]["id"], "@somechannel");
    }

    #[tokio::test]
    async fn it_should_return_204_when_reconciling_an_existing_channel() {
        let (router, _events, _channel_videos) = test_router_with(
            Arc::new(FakeYoutubeChannelRepository {
                resolved: Some(resolved_channel()),
            }),
            vec![ChannelVideoListing {
                youtube_id: "yt1".to_string(),
                title: "One".to_string(),
                position: 0,
            }],
        );
        router
            .clone()
            .oneshot(create_request("@somechannel", "high", 10))
            .await
            .unwrap();

        let response = router
            .oneshot(reconcile_request("@somechannel"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn it_should_not_change_the_number_of_pending_reconcile_tasks_on_repeated_reconcile() {
        let (router, _events, _channel_videos) = test_router_with(
            Arc::new(FakeYoutubeChannelRepository {
                resolved: Some(resolved_channel()),
            }),
            Vec::new(),
        );
        router
            .clone()
            .oneshot(create_request("@somechannel", "high", 10))
            .await
            .unwrap();

        router
            .clone()
            .oneshot(reconcile_request("@somechannel"))
            .await
            .unwrap();
        let response = router
            .oneshot(reconcile_request("@somechannel"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn it_should_return_204_and_do_nothing_when_reconciling_a_nonexistent_channel() {
        let (router, _events) = test_router(Some(resolved_channel()));

        let response = router.oneshot(reconcile_request("@missing")).await.unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn it_should_return_400_when_reconciling_with_an_invalid_handle() {
        let (router, _events) = test_router(Some(resolved_channel()));

        let response = router.oneshot(reconcile_request("noatsign")).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
