pub mod dto;

use super::AppState;
use super::error::error_response;
use super::playlists::dto::PlaylistResponse;
use crate::domain::playlist::{CreateCustomPlaylistError, PlaylistName, PlaylistPath};
use crate::domain::shared::{PlaylistId, Quality, VideoId};
use crate::domain::video::{AddVideoToCustomPlaylistError, RemoveVideoFromPlaylistError};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dto::{AddVideoRequest, CreateCustomPlaylistRequest};

pub async fn create_custom_playlist(
    State(state): State<AppState>,
    Json(request): Json<CreateCustomPlaylistRequest>,
) -> Response {
    let id = match PlaylistId::new(request.id) {
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

    let result = tokio::task::spawn_blocking(move || {
        state
            .playlist_creator
            .create_custom(id, name, path, quality)
    })
    .await;

    match result {
        Ok(Ok(playlist)) => {
            (StatusCode::CREATED, Json(PlaylistResponse::from(playlist))).into_response()
        }
        Ok(Err(e @ CreateCustomPlaylistError::InvalidId(..))) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Ok(Err(e @ CreateCustomPlaylistError::AlreadyExists(_))) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Ok(Err(e @ CreateCustomPlaylistError::PathAlreadyInUse(_))) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Ok(Err(e @ CreateCustomPlaylistError::Repository(_))) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn add_video(
    State(state): State<AppState>,
    Path(playlist_id): Path<String>,
    Json(request): Json<AddVideoRequest>,
) -> Response {
    let playlist_id = match PlaylistId::new(playlist_id) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let video_id = match VideoId::from_url_or_id(request.video) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };

    let result = tokio::task::spawn_blocking(move || {
        state.custom_playlist_video_adder.add(playlist_id, video_id)
    })
    .await;

    match result {
        Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(e @ AddVideoToCustomPlaylistError::PlaylistNotFound(_))) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Ok(Err(e @ AddVideoToCustomPlaylistError::NotCustomPlaylist(_))) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Ok(Err(e @ AddVideoToCustomPlaylistError::YoutubeVideoNotFound(_))) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Ok(Err(e @ AddVideoToCustomPlaylistError::Lookup(_))) => {
            error_response(StatusCode::BAD_GATEWAY, e.to_string())
        }
        Ok(Err(e @ AddVideoToCustomPlaylistError::Repository(_))) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn remove_video(
    State(state): State<AppState>,
    Path((playlist_id, video_id)): Path<(String, String)>,
) -> Response {
    let playlist_id = match PlaylistId::new(playlist_id) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let video_id = match VideoId::new(video_id) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };

    let result = tokio::task::spawn_blocking(move || {
        state
            .custom_playlist_video_remover
            .remove(playlist_id, video_id)
    })
    .await;

    match result {
        Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(e @ RemoveVideoFromPlaylistError::PlaylistNotFound(_))) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Ok(Err(e @ RemoveVideoFromPlaylistError::NotCustomPlaylist(_))) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Ok(Err(e @ RemoveVideoFromPlaylistError::VideoNotFound(_))) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Ok(Err(e @ RemoveVideoFromPlaylistError::Repository(_))) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::DomainEvent;
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::shared::Quality;
    use crate::domain::video::Video;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::FakeChannelRepository;
    use crate::infrastructure::repositories::sqlite_channel_video_repository::FakeChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::{
        FakePlaylistVideoRepository, PlaylistVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_channel_videos_repository::FakeChannelVideosRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::repositories::youtube_playlist_repository::FakeYoutubePlaylistRepository;

    use crate::infrastructure::repositories::youtube_video_repository::{
        FakeYoutubeVideoRepository, YoutubeVideo,
    };
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use axum::routing::{delete, post};
    use chrono::{DateTime, Utc};
    use std::sync::Arc;
    use tower::ServiceExt;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn test_router(
        playlist_repository: Arc<FakePlaylistRepository>,
        video_repository: Arc<FakeVideoRepository>,
        youtube_video: Option<YoutubeVideo>,
    ) -> (
        axum::Router,
        Arc<FakeEventPublisher>,
        Arc<FakePlaylistVideoRepository>,
    ) {
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
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
        let video_reconciler = crate::domain::services::VideoReconciler::new(
            playlist_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            event_publisher.clone() as Arc<dyn crate::infrastructure::shared::domain_events::event_publisher::EventPublisher>,
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let custom_playlist_video_adder = crate::domain::services::CustomPlaylistVideoAdder::new(
            playlist_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeYoutubeVideoRepository {
                video: youtube_video,
            }),
            event_publisher.clone() as Arc<dyn crate::infrastructure::shared::domain_events::event_publisher::EventPublisher>,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let custom_playlist_video_remover =
            crate::domain::services::CustomPlaylistVideoRemover::new(
                playlist_repository.clone(),
                video_repository.clone(),
                playlist_video_repository.clone(),
                event_publisher.clone() as Arc<dyn crate::infrastructure::shared::domain_events::event_publisher::EventPublisher>,
            );
        let video_searcher = crate::domain::services::VideoSearcher::new(
            playlist_repository,
            playlist_video_repository.clone(),
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            video_repository,
        );
        let channel_video_reconciler = crate::domain::services::ChannelVideoReconciler::new(
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeVideoRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            Arc::new(FakeChannelVideosRepository::default()),
            event_publisher.clone() as Arc<dyn crate::infrastructure::shared::domain_events::event_publisher::EventPublisher>,
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let state = AppState {
            playlist_creator,
            playlist_deleter,
            playlist_searcher,
            video_reconciler,
            custom_playlist_video_adder,
            custom_playlist_video_remover,
            video_searcher,
            task_service: crate::domain::task::TaskService::new(Arc::new(
                FakeTaskRepository::default(),
            )),
            channel_service: crate::domain::channel::ChannelService::new(
                Arc::new(crate::infrastructure::repositories::sqlite_channel_repository::FakeChannelRepository::default()),
                Arc::new(crate::infrastructure::repositories::youtube_channel_repository::FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeVideoRepository::default()),
                Arc::new(FakeChannelVideoRepository::default()),
                event_publisher.clone() as Arc<dyn crate::infrastructure::shared::domain_events::event_publisher::EventPublisher>,
                Arc::new(FixedClock(fixed_timestamp())),
            ),
            channel_video_reconciler,
        };
        let inner = Router::new()
            .route("/custom-playlists", post(create_custom_playlist))
            .route("/custom-playlists/{id}/videos", post(add_video))
            .route(
                "/custom-playlists/{id}/videos/{video_id}",
                delete(remove_video),
            )
            .with_state(state);
        let router = Router::new().nest("/api", inner);
        (router, event_publisher, playlist_video_repository)
    }

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    const VALID_UUID: &str = "11111111-1111-1111-1111-111111111111";

    fn create_request(id: &str, name: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/custom-playlists")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "id": id, "name": name, "path": "music/chill", "quality": "high" })
                    .to_string(),
            ))
            .unwrap()
    }

    fn add_video_request(playlist_id: &str, video: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(format!("/api/custom-playlists/{playlist_id}/videos"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "video": video }).to_string(),
            ))
            .unwrap()
    }

    fn remove_video_request(playlist_id: &str, video_id: &str) -> Request<Body> {
        Request::builder()
            .method("DELETE")
            .uri(format!(
                "/api/custom-playlists/{playlist_id}/videos/{video_id}"
            ))
            .body(Body::empty())
            .unwrap()
    }

    fn custom_playlist(id: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new("My Custom Playlist").unwrap(),
            PlaylistPath::new("custom").unwrap(),
            Quality::High,
            PlaylistKind::Custom,
            fixed_timestamp(),
        )
    }

    fn youtube_linked_playlist(id: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new("My YouTube Playlist").unwrap(),
            PlaylistPath::new("yt").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            fixed_timestamp(),
        )
    }

    #[tokio::test]
    async fn it_should_return_201_when_creating_a_custom_playlist() {
        let (router, _events, _playlist_videos) = test_router(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeVideoRepository::default()),
            None,
        );

        let response = router
            .oneshot(create_request(VALID_UUID, "My Playlist"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = body_json(response).await;
        assert_eq!(body["id"], VALID_UUID);
        assert_eq!(body["kind"], "custom");
    }

    #[tokio::test]
    async fn it_should_publish_a_playlist_created_event_on_creation() {
        let (router, events, _playlist_videos) = test_router(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeVideoRepository::default()),
            None,
        );

        router
            .oneshot(create_request(VALID_UUID, "My Playlist"))
            .await
            .unwrap();

        assert_eq!(
            *events.published.lock().unwrap(),
            vec![DomainEvent::PlaylistCreated {
                playlist_id: VALID_UUID.to_string()
            }]
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_id_is_not_a_well_formed_uuid() {
        let (router, _events, _playlist_videos) = test_router(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeVideoRepository::default()),
            None,
        );

        let response = router
            .oneshot(create_request("not-a-uuid", "My Playlist"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_id_already_identifies_a_playlist() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert(&custom_playlist(VALID_UUID))
            .unwrap();
        let (router, _events, _playlist_videos) = test_router(
            playlist_repository,
            Arc::new(FakeVideoRepository::default()),
            None,
        );

        let response = router
            .oneshot(create_request(VALID_UUID, "My Playlist"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_name_is_invalid() {
        let (router, _events, _playlist_videos) = test_router(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeVideoRepository::default()),
            None,
        );

        let response = router
            .oneshot(create_request(VALID_UUID, ""))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_is_missing() {
        let (router, _events, _playlist_videos) = test_router(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeVideoRepository::default()),
            None,
        );

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/custom-playlists")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "id": VALID_UUID, "name": "My Playlist", "quality": "high" })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_quality_is_invalid() {
        let (router, _events, _playlist_videos) = test_router(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeVideoRepository::default()),
            None,
        );

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/custom-playlists")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "id": VALID_UUID, "name": "My Playlist", "path": "custom", "quality": "ultra" })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_204_and_store_a_pending_video_on_successful_add() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&custom_playlist("PL1")).unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        let (router, events, _playlist_videos) = test_router(
            playlist_repository,
            video_repository.clone(),
            Some(YoutubeVideo {
                video_id: "vid1".to_string(),
                title: "My Video".to_string(),
            }),
        );

        let response = router
            .oneshot(add_video_request("PL1", "vid1"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].title, "My Video");
        assert_eq!(
            *events.published.lock().unwrap(),
            vec![DomainEvent::VideoAddedToPlaylist {
                playlist_id: "PL1".to_string(),
                video_id: stored[0].id.as_str().to_string(),
            }]
        );
    }

    #[tokio::test]
    async fn it_should_accept_a_full_youtube_url_for_add_video() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&custom_playlist("PL1")).unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        let (router, _events, _playlist_videos) = test_router(
            playlist_repository,
            video_repository.clone(),
            Some(YoutubeVideo {
                video_id: "vid1".to_string(),
                title: "My Video".to_string(),
            }),
        );

        let response = router
            .oneshot(add_video_request(
                "PL1",
                "https://www.youtube.com/watch?v=vid1",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(video_repository.videos.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn it_should_not_publish_a_second_event_when_the_video_is_already_a_member() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&custom_playlist("PL1")).unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        let (router, events, playlist_video_repository) = test_router(
            playlist_repository,
            video_repository.clone(),
            Some(YoutubeVideo {
                video_id: "vid1".to_string(),
                title: "My Video".to_string(),
            }),
        );
        let existing = Video::create(
            crate::domain::shared::VideoId::new("vid1").unwrap(),
            "Existing",
            fixed_timestamp(),
        );
        video_repository.save(&existing).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                existing.id.clone(),
                fixed_timestamp(),
            ))
            .unwrap();
        playlist_video_repository.register_youtube_id(&existing.id, &existing.youtube_id);

        let response = router
            .oneshot(add_video_request("PL1", "vid1"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(events.published.lock().unwrap().is_empty());
        assert_eq!(video_repository.videos.lock().unwrap()[0].title, "Existing");
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_video_does_not_exist_on_youtube() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&custom_playlist("PL1")).unwrap();
        let (router, _events, _playlist_videos) = test_router(
            playlist_repository,
            Arc::new(FakeVideoRepository::default()),
            None,
        );

        let response = router
            .oneshot(add_video_request("PL1", "vid1"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_video_value_is_malformed() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&custom_playlist("PL1")).unwrap();
        let (router, _events, _playlist_videos) = test_router(
            playlist_repository,
            Arc::new(FakeVideoRepository::default()),
            None,
        );

        let response = router
            .oneshot(add_video_request("PL1", "https://example.com/not-youtube"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_target_playlist_does_not_exist_for_add() {
        let (router, _events, _playlist_videos) = test_router(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeVideoRepository::default()),
            Some(YoutubeVideo {
                video_id: "vid1".to_string(),
                title: "My Video".to_string(),
            }),
        );

        let response = router
            .oneshot(add_video_request("PL404", "vid1"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_target_playlist_is_youtube_linked_for_add() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert(&youtube_linked_playlist("PL1"))
            .unwrap();
        let (router, _events, _playlist_videos) = test_router(
            playlist_repository,
            Arc::new(FakeVideoRepository::default()),
            Some(YoutubeVideo {
                video_id: "vid1".to_string(),
                title: "My Video".to_string(),
            }),
        );

        let response = router
            .oneshot(add_video_request("PL1", "vid1"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_204_and_delete_the_record_on_successful_removal() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&custom_playlist("PL1")).unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        let (router, events, playlist_video_repository) =
            test_router(playlist_repository, video_repository.clone(), None);
        let existing = Video::create(
            crate::domain::shared::VideoId::new("vid1").unwrap(),
            "My Video",
            fixed_timestamp(),
        );
        video_repository.save(&existing).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                existing.id.clone(),
                fixed_timestamp(),
            ))
            .unwrap();
        playlist_video_repository.register_youtube_id(&existing.id, &existing.youtube_id);

        let response = router
            .oneshot(remove_video_request("PL1", "vid1"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(video_repository.videos.lock().unwrap().is_empty());
        assert_eq!(
            *events.published.lock().unwrap(),
            vec![DomainEvent::VideoRemovedFromPlaylist {
                playlist_id: "PL1".to_string(),
                video_id: existing.id.as_str().to_string(),
                title: "My Video".to_string(),
                filename: None,
                was_downloaded: false,
            }]
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_video_is_not_a_member_for_removal() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&custom_playlist("PL1")).unwrap();
        let (router, _events, _playlist_videos) = test_router(
            playlist_repository,
            Arc::new(FakeVideoRepository::default()),
            None,
        );

        let response = router
            .oneshot(remove_video_request("PL1", "vid1"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_target_playlist_does_not_exist_for_removal() {
        let (router, _events, _playlist_videos) = test_router(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeVideoRepository::default()),
            None,
        );

        let response = router
            .oneshot(remove_video_request("PL404", "vid1"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_target_playlist_is_youtube_linked_for_removal() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert(&youtube_linked_playlist("PL1"))
            .unwrap();
        let (router, _events, _playlist_videos) = test_router(
            playlist_repository,
            Arc::new(FakeVideoRepository::default()),
            None,
        );

        let response = router
            .oneshot(remove_video_request("PL1", "vid1"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
