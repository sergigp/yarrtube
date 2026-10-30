pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::preferences::Preferences;
use crate::domain::services::{
    PreferencesSearcher, PreferencesSearcherApi, PreferencesUpdater, PreferencesUpdaterApi,
};
use axum::Json;
use axum::extract::State;
use dto::{PreferencesResponse, UpdatePreferencesRequest};

pub async fn get_preferences(
    State(preferences_searcher): State<PreferencesSearcher>,
) -> Result<Json<PreferencesResponse>, ApiError> {
    let preferences = run_blocking(move || preferences_searcher.get())
        .await?
        .map_err(ApiError::internal)?;
    Ok(Json(PreferencesResponse::from(preferences)))
}

pub async fn update_preferences(
    State(preferences_updater): State<PreferencesUpdater>,
    Json(_request): Json<UpdatePreferencesRequest>,
) -> Result<Json<PreferencesResponse>, ApiError> {
    let preferences = Preferences::default();
    let saved = run_blocking(move || preferences_updater.save(preferences))
        .await?
        .map_err(ApiError::internal)?;
    Ok(Json(PreferencesResponse::from(saved)))
}
