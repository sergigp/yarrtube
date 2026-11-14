pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::services::{
    PlexStatusSearcher, PlexStatusSearcherApi, PlexSyncRequester, PlexSyncRequesterApi,
};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use dto::PlexStatusResponse;

pub async fn get_plex_status(
    State(plex_status_searcher): State<PlexStatusSearcher>,
) -> Result<Json<PlexStatusResponse>, ApiError> {
    let status = run_blocking(move || plex_status_searcher.get())
        .await?
        .map_err(ApiError::internal)?;
    Ok(Json(PlexStatusResponse::from(status)))
}

pub async fn request_plex_sync(
    State(_plex_sync_requester): State<PlexSyncRequester>,
    Path(_kind): Path<String>,
) -> Result<StatusCode, ApiError> {
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
