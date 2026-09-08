pub mod dto;

use super::AppState;
use super::error::error_response;
use crate::domain::playlist::{
    CreatePlaylistError, CreatePlaylistOutcome, DeletePlaylistError, PlaylistName,
};
use crate::domain::shared::PlaylistId;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dto::{CreatePlaylistRequest, PlaylistResponse};

pub async fn create_playlist(
    State(state): State<AppState>,
    Json(request): Json<CreatePlaylistRequest>,
) -> Response {
    let id = match PlaylistId::new(request.id) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let name = match PlaylistName::new(request.name) {
        Ok(name) => name,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };

    let result =
        tokio::task::spawn_blocking(move || state.playlist_service.create_playlist(id, name)).await;

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

    match state.playlist_service.delete_playlist(id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e @ DeletePlaylistError::NotFound(_)) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Err(e @ DeletePlaylistError::Repository(_)) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    }
}

pub async fn list_playlists(State(state): State<AppState>) -> Response {
    match state.playlist_service.list_playlists() {
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
    use crate::domain::event::DomainEvent;
    use crate::http::playlists_router;
    use crate::infrastructure::repositories::sqlite_playlist_repository::FakePlaylistRepository;
    use crate::infrastructure::repositories::system_clock::FixedClock;
    use crate::infrastructure::repositories::youtube_playlist_repository::FakeYoutubePlaylistRepository;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;
    use tower::ServiceExt;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn test_router(
        repository: FakePlaylistRepository,
        youtube_exists: bool,
    ) -> (axum::Router, Arc<FakePlaylistRepository>) {
        let repository = Arc::new(repository);
        let state = AppState {
            playlist_service: crate::domain::playlist::PlaylistService::new(
                repository.clone(),
                Arc::new(FakeYoutubePlaylistRepository {
                    exists: youtube_exists,
                }),
                Arc::new(FixedClock(fixed_timestamp())),
            ),
        };
        (playlists_router(state), repository)
    }

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn create_request(id: &str, name: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/playlists")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "id": id, "name": name }).to_string(),
            ))
            .unwrap()
    }

    #[tokio::test]
    async fn it_should_return_201_when_creating_a_new_playlist() {
        let (router, _repository) = test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(create_request("PL1", "My Playlist"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = body_json(response).await;
        assert_eq!(body["id"], "PL1");
        assert_eq!(body["name"], "My Playlist");
        assert_eq!(
            body["created_at"],
            fixed_timestamp().to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
        );
    }

    #[tokio::test]
    async fn it_should_return_200_when_creating_a_playlist_that_already_exists() {
        let (router, _repository) = test_router(FakePlaylistRepository::default(), true);
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
        let (router, _repository) = test_router(FakePlaylistRepository::default(), true);

        let response = router.oneshot(create_request("PL1", "")).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_youtube_playlist_does_not_exist() {
        let (router, _repository) = test_router(FakePlaylistRepository::default(), false);

        let response = router
            .oneshot(create_request("PL404", "My Playlist"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_record_a_playlist_created_event_only_once_for_repeated_creation() {
        let (router, repository) = test_router(FakePlaylistRepository::default(), true);
        router
            .clone()
            .oneshot(create_request("PL1", "First"))
            .await
            .unwrap();

        router
            .oneshot(create_request("PL1", "First Again"))
            .await
            .unwrap();

        let published = repository.transactional_events.lock().unwrap();
        assert_eq!(
            *published,
            vec![DomainEvent::PlaylistCreated {
                playlist_id: "PL1".to_string()
            }]
        );
    }

    #[tokio::test]
    async fn it_should_return_204_when_deleting_an_existing_playlist() {
        let (router, _repository) = test_router(FakePlaylistRepository::default(), true);
        router
            .clone()
            .oneshot(create_request("PL1", "My Playlist"))
            .await
            .unwrap();

        let response = router
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/playlists/PL1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn it_should_record_a_playlist_deleted_event_on_successful_deletion() {
        let (router, repository) = test_router(FakePlaylistRepository::default(), true);
        router
            .clone()
            .oneshot(create_request("PL1", "My Playlist"))
            .await
            .unwrap();

        router
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/playlists/PL1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let published = repository.transactional_events.lock().unwrap();
        assert_eq!(
            *published,
            vec![
                DomainEvent::PlaylistCreated {
                    playlist_id: "PL1".to_string()
                },
                DomainEvent::PlaylistDeleted {
                    playlist_id: "PL1".to_string()
                },
            ]
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_deleting_a_missing_playlist() {
        let (router, _repository) = test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/playlists/PL404")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_an_empty_array_when_no_playlists_exist() {
        let (router, _repository) = test_router(FakePlaylistRepository::default(), true);

        let response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/playlists")
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
        let (router, _repository) = test_router(FakePlaylistRepository::default(), true);
        router
            .clone()
            .oneshot(create_request("PL1", "First"))
            .await
            .unwrap();
        router
            .clone()
            .oneshot(create_request("PL2", "Second"))
            .await
            .unwrap();

        let response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/playlists")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body.as_array().unwrap().len(), 2);
    }
}
