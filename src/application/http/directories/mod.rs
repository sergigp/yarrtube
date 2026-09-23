pub mod dto;

use super::VideosRoot;
use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::directory::{DirectoryPath, ListDirectoriesError};
use crate::domain::services::DirectorySearcher;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use dto::{DirectoryResponse, ListDirectoriesQuery};

pub async fn list_directories(
    State(directory_searcher): State<DirectorySearcher>,
    State(VideosRoot(videos_root)): State<VideosRoot>,
    Query(query): Query<ListDirectoriesQuery>,
) -> Result<Json<DirectoryResponse>, ApiError> {
    let path = match query.path {
        None => DirectoryPath::root(),
        Some(path) => DirectoryPath::new(path).map_err(ApiError::bad_request)?,
    };

    match run_blocking(move || directory_searcher.list(&path)).await? {
        Ok(directory) => Ok(Json(DirectoryResponse::new(&videos_root, directory))),
        Err(e @ ListDirectoriesError::NotFound(_)) => Err(ApiError::new(StatusCode::NOT_FOUND, e)),
        Err(e @ ListDirectoriesError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::filesystem_directory_repository::FakeDirectoryRepository;
    use dto::DirectoryEntryResponse;
    use std::sync::Arc;

    const VIDEOS_ROOT: &str = "/videos";

    #[tokio::test]
    async fn it_should_return_the_videos_roots_subdirectories_on_a_request_without_a_path() {
        let response = list(seeded_root(), None).await;

        assert_eq!(
            response,
            Ok(directory_response("", &["channels", "playlists"]))
        );
    }

    #[tokio::test]
    async fn it_should_return_a_nested_directorys_subdirectories() {
        let response = list(seeded_root(), Some("playlists")).await;

        assert_eq!(response, Ok(directory_response("playlists", &["music"])));
    }

    #[tokio::test]
    async fn it_should_return_an_empty_entry_list_on_a_directory_with_no_subdirectories() {
        let response = list(seeded_root(), Some("channels")).await;

        assert_eq!(response, Ok(directory_response("channels", &[])));
    }

    #[tokio::test]
    async fn it_should_return_bad_request_on_a_path_with_a_parent_traversal_segment() {
        let response = list(seeded_root(), Some("playlists/../..")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Directory path must not contain \"..\" segments"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_bad_request_on_an_absolute_path() {
        let response = list(seeded_root(), Some("/etc")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Directory path must not be an absolute path"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_not_found_on_a_path_the_repository_cannot_list() {
        let response = list(seeded_root(), Some("does-not-exist")).await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "no directory \"does-not-exist\" exists under the videos root"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_internal_server_error_on_a_repository_failure() {
        let response = list(
            FakeDirectoryRepository::failing("the videos root is unreadable"),
            None,
        )
        .await;

        assert_eq!(
            response,
            Err(ApiError::internal("the videos root is unreadable"))
        );
    }

    async fn list(
        repository: FakeDirectoryRepository,
        path: Option<&str>,
    ) -> Result<DirectoryResponse, ApiError> {
        list_directories(
            State(DirectorySearcher::new(Arc::new(repository))),
            State(VideosRoot(VIDEOS_ROOT.to_string())),
            Query(ListDirectoriesQuery {
                path: path.map(str::to_string),
            }),
        )
        .await
        .map(|Json(directory)| directory)
    }

    fn directory_response(path: &str, entries: &[&str]) -> DirectoryResponse {
        DirectoryResponse {
            root: VIDEOS_ROOT.to_string(),
            path: path.to_string(),
            entries: entries
                .iter()
                .map(|name| DirectoryEntryResponse {
                    name: name.to_string(),
                })
                .collect(),
        }
    }

    fn seeded_root() -> FakeDirectoryRepository {
        FakeDirectoryRepository::with_directories(&[
            ("", &["channels", "playlists"]),
            ("playlists", &["music"]),
            ("channels", &[]),
        ])
    }
}
