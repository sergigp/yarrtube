pub mod dto;

use super::VideosRoot;
use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::directory::{DirectoryPath, ListDirectoriesError};
use crate::domain::services::DirectorySearcher;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dto::{DirectoryResponse, ListDirectoriesQuery};

pub async fn list_directories(
    State(directory_searcher): State<DirectorySearcher>,
    State(VideosRoot(videos_root)): State<VideosRoot>,
    Query(query): Query<ListDirectoriesQuery>,
) -> Result<Response, ApiError> {
    let path = match query.path {
        None => DirectoryPath::root(),
        Some(path) => DirectoryPath::new(path).map_err(ApiError::bad_request)?,
    };

    match run_blocking(move || directory_searcher.list(&path)).await? {
        Ok(directory) => Ok((
            StatusCode::OK,
            Json(DirectoryResponse::new(&videos_root, directory)),
        )
            .into_response()),
        Err(e @ ListDirectoriesError::NotFound(_)) => Err(ApiError::new(StatusCode::NOT_FOUND, e)),
        Err(e @ ListDirectoriesError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::filesystem_directory_repository::FakeDirectoryRepository;
    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::extract::FromRef;
    use axum::http::Request;
    use axum::routing::get;
    use std::sync::Arc;
    use tower::ServiceExt;

    const VIDEOS_ROOT: &str = "/videos";

    #[derive(Clone, FromRef)]
    struct TestState {
        directory_searcher: DirectorySearcher,
        videos_root: VideosRoot,
    }

    fn test_router(repository: FakeDirectoryRepository) -> Router {
        let state = TestState {
            directory_searcher: DirectorySearcher::new(Arc::new(repository)),
            videos_root: VideosRoot(VIDEOS_ROOT.to_string()),
        };
        Router::new()
            .route("/api/directories", get(list_directories))
            .with_state(state)
    }

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn list(repository: FakeDirectoryRepository, uri: &str) -> Response {
        test_router(repository)
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    fn seeded_root() -> FakeDirectoryRepository {
        FakeDirectoryRepository::with_directories(&[
            ("", &["channels", "playlists"]),
            ("playlists", &["music"]),
            ("channels", &[]),
        ])
    }

    #[tokio::test]
    async fn it_should_return_the_videos_roots_subdirectories_on_a_request_without_a_path() {
        let response = list(seeded_root(), "/api/directories").await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["path"], "");
        assert_eq!(
            body["entries"],
            serde_json::json!([{ "name": "channels" }, { "name": "playlists" }])
        );
    }

    #[tokio::test]
    async fn it_should_return_a_nested_directorys_subdirectories() {
        let response = list(seeded_root(), "/api/directories?path=playlists").await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["path"], "playlists");
        assert_eq!(body["entries"], serde_json::json!([{ "name": "music" }]));
    }

    #[tokio::test]
    async fn it_should_return_an_empty_entry_list_on_a_directory_with_no_subdirectories() {
        let response = list(seeded_root(), "/api/directories?path=channels").await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["entries"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn it_should_report_the_videos_root_on_every_successful_listing() {
        let root_listing = list(seeded_root(), "/api/directories").await;
        let nested_listing = list(seeded_root(), "/api/directories?path=playlists").await;

        assert_eq!(body_json(root_listing).await["root"], VIDEOS_ROOT);
        assert_eq!(body_json(nested_listing).await["root"], VIDEOS_ROOT);
    }

    #[tokio::test]
    async fn it_should_return_bad_request_on_a_path_with_a_parent_traversal_segment() {
        let response = list(seeded_root(), "/api/directories?path=playlists/../..").await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_json(response).await;
        assert_eq!(
            body["error"],
            "Directory path must not contain \"..\" segments"
        );
    }

    #[tokio::test]
    async fn it_should_return_bad_request_on_an_absolute_path() {
        let response = list(seeded_root(), "/api/directories?path=%2Fetc").await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_json(response).await;
        assert_eq!(body["error"], "Directory path must not be an absolute path");
    }

    #[tokio::test]
    async fn it_should_return_not_found_on_a_path_the_repository_cannot_list() {
        let response = list(seeded_root(), "/api/directories?path=does-not-exist").await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = body_json(response).await;
        assert_eq!(
            body["error"],
            "no directory \"does-not-exist\" exists under the videos root"
        );
    }

    #[tokio::test]
    async fn it_should_return_internal_server_error_on_a_repository_failure() {
        let response = list(
            FakeDirectoryRepository::failing("the videos root is unreadable"),
            "/api/directories",
        )
        .await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = body_json(response).await;
        assert_eq!(body["error"], "the videos root is unreadable");
    }
}
