pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::plex_status::PlexSyncKind;
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
    State(plex_sync_requester): State<PlexSyncRequester>,
    Path(kind): Path<String>,
) -> Result<StatusCode, ApiError> {
    let kind = PlexSyncKind::new(&kind)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, format!("No Plex sync {kind}")))?;
    run_blocking(move || plex_sync_requester.request(kind))
        .await?
        .map_err(ApiError::internal)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
