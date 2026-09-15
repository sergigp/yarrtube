pub mod dto;

use super::AppState;
use super::error::error_response;
use crate::domain::channel::{
    ChannelHandle, CreateChannelError, CreateChannelOutcome, DeleteChannelError, VideoLimit,
};
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

    let result = tokio::task::spawn_blocking(move || {
        state
            .channel_service
            .create_channel(id, quality, video_limit)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::ChannelService;
    use crate::domain::event::DomainEvent;
    use crate::domain::task::TaskService;
    use crate::domain::video::VideoService;
    use crate::http::api_router;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::FakeChannelRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::FakePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::youtube_channel_repository::{
        FakeYoutubeChannelRepository, ResolvedChannel, YoutubeChannelRepository,
    };
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::repositories::youtube_playlist_repository::FakeYoutubePlaylistRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::repositories::youtube_video_repository::FakeYoutubeVideoRepository;
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
        }
    }

    fn test_router_with_lookup(
        lookup: Arc<dyn YoutubeChannelRepository>,
    ) -> (axum::Router, Arc<FakeEventPublisher>) {
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_repository = Arc::new(FakeTaskRepository::default());
        let playlist_service = crate::domain::playlist::PlaylistService::new(
            playlist_repository.clone(),
            video_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher.clone() as Arc<dyn crate::infrastructure::shared::domain_events::event_publisher::EventPublisher>,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let video_service = VideoService::new(
            playlist_repository,
            video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeYoutubeVideoRepository::default()),
            event_publisher.clone() as Arc<dyn crate::infrastructure::shared::domain_events::event_publisher::EventPublisher>,
            task_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let channel_service = ChannelService::new(
            Arc::new(FakeChannelRepository::default()),
            lookup,
            event_publisher.clone() as Arc<dyn crate::infrastructure::shared::domain_events::event_publisher::EventPublisher>,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let state = AppState {
            playlist_service,
            video_service,
            task_service: TaskService::new(task_repository),
            channel_service,
        };
        (
            axum::Router::new().nest("/api", api_router(state)),
            event_publisher,
        )
    }

    fn test_router(resolved: Option<ResolvedChannel>) -> (axum::Router, Arc<FakeEventPublisher>) {
        test_router_with_lookup(Arc::new(FakeYoutubeChannelRepository { resolved }))
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
        Request::builder()
            .method("POST")
            .uri("/api/channels")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "channel": channel, "quality": quality, "video_limit": video_limit })
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
    }

    #[tokio::test]
    async fn it_should_return_201_when_creating_a_channel_from_a_url() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request(
                "https://www.youtube.com/@somechannel",
                "high",
                10,
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
            .oneshot(create_request("@somechannel", "low", 5))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["quality"], "high");
        assert_eq!(body["video_limit"], 10);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_channel_value_is_missing() {
        let (router, _event_publisher) = test_router(Some(resolved_channel()));

        let response = router
            .oneshot(create_request_body(
                serde_json::json!({ "quality": "high", "video_limit": 10 }),
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
        let (router, _event_publisher) =
            test_router_with_lookup(Arc::new(FailingYoutubeChannelRepository));

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
                serde_json::json!({ "channel": "@somechannel", "video_limit": 10 }),
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
                serde_json::json!({ "channel": "@somechannel", "quality": "high" }),
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
                    channel_id: "@somechannel".to_string()
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
}
