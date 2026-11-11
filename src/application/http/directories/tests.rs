use super::*;
use crate::infrastructure::repositories::filesystem_directory_repository::FakeDirectoryRepository;
use dto::DirectoryEntryResponse;
use std::sync::Arc;

const VIDEOS_ROOT: &str = "/videos";

#[tokio::test]
async fn it_should_list_root_subdirectories_by_default() {
    let directory_searcher = DirectorySearcher::new(Arc::new(seeded_root()));
    let query = ListDirectoriesQuery { path: None };

    let response = list(directory_searcher, query).await;

    assert_eq!(
        response,
        Ok(directory_response("", &["channels", "playlists"]))
    );
}

#[tokio::test]
async fn it_should_list_nested_subdirectories() {
    let directory_searcher = DirectorySearcher::new(Arc::new(seeded_root()));
    let query = ListDirectoriesQuery {
        path: Some("playlists".to_string()),
    };

    let response = list(directory_searcher, query).await;

    assert_eq!(response, Ok(directory_response("playlists", &["music"])));
}

#[tokio::test]
async fn it_should_list_no_entries_for_a_leaf_directory() {
    let directory_searcher = DirectorySearcher::new(Arc::new(seeded_root()));
    let query = ListDirectoriesQuery {
        path: Some("channels".to_string()),
    };

    let response = list(directory_searcher, query).await;

    assert_eq!(response, Ok(directory_response("channels", &[])));
}

#[tokio::test]
async fn it_should_fail_if_invalid_path_provided() {
    let query = ListDirectoriesQuery {
        path: Some("playlists/../..".to_string()),
    };

    let response = list(any_directory_searcher(), query).await;

    assert_eq!(
        response,
        Err(ApiError::bad_request(
            "Directory path must not contain \"..\" segments"
        ))
    );
}

#[tokio::test]
async fn it_should_fail_if_path_not_found() {
    let directory_searcher = DirectorySearcher::new(Arc::new(seeded_root()));
    let query = ListDirectoriesQuery {
        path: Some("does-not-exist".to_string()),
    };

    let response = list(directory_searcher, query).await;

    assert_eq!(
        response,
        Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "no directory \"does-not-exist\" exists under the videos root"
        ))
    );
}

#[tokio::test]
async fn it_should_fail_if_repository_fails() {
    let directory_searcher = DirectorySearcher::new(Arc::new(FakeDirectoryRepository::failing(
        "the videos root is unreadable",
    )));
    let query = ListDirectoriesQuery { path: None };

    let response = list(directory_searcher, query).await;

    assert_eq!(
        response,
        Err(ApiError::internal("the videos root is unreadable"))
    );
}

/// A searcher for tests whose request is rejected before reaching it.
fn any_directory_searcher() -> DirectorySearcher {
    DirectorySearcher::new(Arc::new(FakeDirectoryRepository::with_directories(&[])))
}

fn seeded_root() -> FakeDirectoryRepository {
    FakeDirectoryRepository::with_directories(&[
        ("", &["channels", "playlists"]),
        ("playlists", &["music"]),
        ("channels", &[]),
    ])
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

async fn list(
    directory_searcher: DirectorySearcher,
    query: ListDirectoriesQuery,
) -> Result<DirectoryResponse, ApiError> {
    list_directories(
        State(directory_searcher),
        State(VideosRoot(VIDEOS_ROOT.to_string())),
        Query(query),
    )
    .await
    .map(|Json(directory)| directory)
}
