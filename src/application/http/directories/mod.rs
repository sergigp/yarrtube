pub mod dto;

use super::VideosRoot;
use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::directory::{DirectoryPath, ListDirectoriesError};
use crate::domain::services::{DirectorySearcher, DirectorySearcherApi};
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
        Some(path) => DirectoryPath::new(path)?,
    };

    match run_blocking(move || directory_searcher.list(&path)).await? {
        Ok(directory) => Ok(Json(DirectoryResponse::new(&videos_root, directory))),
        Err(e @ ListDirectoriesError::NotFound(_)) => Err(ApiError::new(StatusCode::NOT_FOUND, e)),
        Err(e @ ListDirectoriesError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

#[cfg(test)]
mod tests;
