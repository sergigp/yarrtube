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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::sqlite_preferences_repository::SqlitePreferencesRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use std::sync::Arc;

    #[tokio::test]
    async fn it_should_return_the_defaults_if_nothing_saved() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let preferences_searcher = PreferencesSearcher::new(repository);

        let response = get(preferences_searcher).await;

        assert_eq!(response, Ok(all_on_response()));
    }

    async fn get(
        preferences_searcher: PreferencesSearcher,
    ) -> Result<PreferencesResponse, ApiError> {
        get_preferences(State(preferences_searcher))
            .await
            .map(|Json(response)| response)
    }

    fn all_on_response() -> PreferencesResponse {
        PreferencesResponse {
            categories_enabled: true,
            home_show_continue_watching: true,
            home_show_categories: true,
            home_show_category_colors: true,
            home_show_quick_watches: true,
            home_show_latest: true,
        }
    }
}
