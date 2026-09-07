use super::AppState;
use crate::domain::playlist::{Playlist, PlaylistName, YoutubePlaylistId};
use crate::domain::ports::SaveOutcome;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreatePlaylistRequest {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PlaylistResponse {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl From<Playlist> for PlaylistResponse {
    fn from(playlist: Playlist) -> Self {
        Self {
            id: playlist.id.as_str().to_string(),
            name: playlist.name.as_str().to_string(),
            created_at: playlist.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

fn error_response(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
        .into_response()
}

pub async fn create_playlist(
    State(state): State<AppState>,
    Json(request): Json<CreatePlaylistRequest>,
) -> Response {
    let id = match YoutubePlaylistId::new(request.id) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let name = match PlaylistName::new(request.name) {
        Ok(name) => name,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };

    match state.lookup.exists(&id) {
        Ok(true) => {}
        Ok(false) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                format!("YouTube playlist {id} does not exist or is not accessible"),
            );
        }
        Err(e) => return error_response(StatusCode::BAD_GATEWAY, e.to_string()),
    }

    let playlist = Playlist::create(id, name, state.clock.now());

    match state.repository.save(&playlist) {
        Ok(SaveOutcome::Created(playlist)) => {
            (StatusCode::CREATED, Json(PlaylistResponse::from(playlist))).into_response()
        }
        Ok(SaveOutcome::AlreadyExisted(playlist)) => {
            (StatusCode::OK, Json(PlaylistResponse::from(playlist))).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn delete_playlist(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let id = match YoutubePlaylistId::new(id) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };

    match state.repository.delete(&id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => error_response(StatusCode::BAD_REQUEST, format!("playlist {id} not found")),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn list_playlists(State(state): State<AppState>) -> Response {
    match state.repository.list() {
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
    use crate::domain::ports::{
        LookupError, PlaylistRepository, RepositoryError, YoutubePlaylistLookup,
    };
    use crate::http::playlists_router;
    use crate::infra::system_clock::FixedClock;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    #[derive(Default)]
    struct FakePlaylistRepository {
        playlists: Mutex<Vec<Playlist>>,
    }

    impl PlaylistRepository for FakePlaylistRepository {
        fn save(&self, playlist: &Playlist) -> Result<SaveOutcome, RepositoryError> {
            let mut playlists = self.playlists.lock().unwrap();
            if let Some(existing) = playlists.iter().find(|p| p.id == playlist.id) {
                return Ok(SaveOutcome::AlreadyExisted(existing.clone()));
            }
            playlists.push(playlist.clone());
            Ok(SaveOutcome::Created(playlist.clone()))
        }

        fn delete(&self, id: &YoutubePlaylistId) -> Result<bool, RepositoryError> {
            let mut playlists = self.playlists.lock().unwrap();
            let len_before = playlists.len();
            playlists.retain(|p| p.id != *id);
            Ok(playlists.len() != len_before)
        }

        fn list(&self) -> Result<Vec<Playlist>, RepositoryError> {
            Ok(self.playlists.lock().unwrap().clone())
        }
    }

    struct FakeYoutubePlaylistLookup {
        exists: bool,
    }

    impl YoutubePlaylistLookup for FakeYoutubePlaylistLookup {
        fn exists(&self, _id: &YoutubePlaylistId) -> Result<bool, LookupError> {
            Ok(self.exists)
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn test_router(repository: FakePlaylistRepository, youtube_exists: bool) -> axum::Router {
        let state = AppState {
            repository: Arc::new(repository),
            lookup: Arc::new(FakeYoutubePlaylistLookup {
                exists: youtube_exists,
            }),
            clock: Arc::new(FixedClock(fixed_timestamp())),
        };
        playlists_router(state)
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
    async fn create_playlist_succeeds() {
        let router = test_router(FakePlaylistRepository::default(), true);

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
    async fn create_playlist_is_idempotent_on_duplicate_id() {
        let router = test_router(FakePlaylistRepository::default(), true);
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
    async fn create_playlist_rejects_invalid_name() {
        let router = test_router(FakePlaylistRepository::default(), true);

        let response = router.oneshot(create_request("PL1", "")).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_playlist_rejects_nonexistent_youtube_id() {
        let router = test_router(FakePlaylistRepository::default(), false);

        let response = router
            .oneshot(create_request("PL404", "My Playlist"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn delete_playlist_succeeds() {
        let router = test_router(FakePlaylistRepository::default(), true);
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
    async fn delete_playlist_reports_bad_request_for_missing_id() {
        let router = test_router(FakePlaylistRepository::default(), true);

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
    async fn list_playlists_returns_empty_array_when_none_created() {
        let router = test_router(FakePlaylistRepository::default(), true);

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
    async fn list_playlists_returns_all_created_playlists() {
        let router = test_router(FakePlaylistRepository::default(), true);
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

    #[test]
    fn fake_repository_save_then_list_returns_saved_playlist() {
        let repo = FakePlaylistRepository::default();
        let playlist = Playlist::create(
            YoutubePlaylistId::new("PL1").unwrap(),
            PlaylistName::new("First").unwrap(),
            fixed_timestamp(),
        );

        repo.save(&playlist).unwrap();

        assert_eq!(repo.list().unwrap(), vec![playlist]);
    }

    #[test]
    fn fake_repository_save_then_delete_removes_playlist() {
        let repo = FakePlaylistRepository::default();
        let playlist = Playlist::create(
            YoutubePlaylistId::new("PL1").unwrap(),
            PlaylistName::new("First").unwrap(),
            fixed_timestamp(),
        );
        repo.save(&playlist).unwrap();

        assert!(repo.delete(&playlist.id).unwrap());
        assert_eq!(repo.list().unwrap(), Vec::new());
    }
}
