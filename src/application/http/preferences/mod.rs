pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use super::validation::required;
use crate::domain::preferences::Preferences;
use crate::domain::services::{
    PreferencesSearcher, PreferencesSearcherApi, PreferencesUpdater, PreferencesUpdaterApi,
};
use axum::Json;
use axum::extract::State;
use dto::{PreferencesResponse, UpdatePreferencesRequest};

const MISSING_CATEGORIES_ENABLED: &str = "Whether categories are enabled must be stated (missing)";
const MISSING_HOME_SHOW_CONTINUE_WATCHING: &str =
    "Whether to show continue watching on home must be stated (missing)";
const MISSING_HOME_SHOW_CATEGORIES: &str =
    "Whether to show categories on home must be stated (missing)";
const MISSING_HOME_SHOW_CATEGORY_COLORS: &str =
    "Whether to show category colors on home must be stated (missing)";
const MISSING_HOME_SHOW_QUICK_WATCHES: &str =
    "Whether to show quick watches on home must be stated (missing)";
const MISSING_HOME_SHOW_LATEST: &str =
    "Whether to show latest videos on home must be stated (missing)";

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
    Json(request): Json<UpdatePreferencesRequest>,
) -> Result<Json<PreferencesResponse>, ApiError> {
    let preferences = Preferences {
        categories_enabled: required(request.categories_enabled, MISSING_CATEGORIES_ENABLED)?,
        home_show_continue_watching: required(
            request.home_show_continue_watching,
            MISSING_HOME_SHOW_CONTINUE_WATCHING,
        )?,
        home_show_categories: required(request.home_show_categories, MISSING_HOME_SHOW_CATEGORIES)?,
        home_show_category_colors: required(
            request.home_show_category_colors,
            MISSING_HOME_SHOW_CATEGORY_COLORS,
        )?,
        home_show_quick_watches: required(
            request.home_show_quick_watches,
            MISSING_HOME_SHOW_QUICK_WATCHES,
        )?,
        home_show_latest: required(request.home_show_latest, MISSING_HOME_SHOW_LATEST)?,
    };
    let saved = run_blocking(move || preferences_updater.save(preferences))
        .await?
        .map_err(ApiError::internal)?;
    Ok(Json(PreferencesResponse::from(saved)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::sqlite_preferences_repository::{
        PreferencesRepository, SqlitePreferencesRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::{Database, TestDatabase};
    use rusqlite::Connection;
    use std::sync::Arc;

    #[tokio::test]
    async fn it_should_return_the_defaults_if_nothing_saved() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let preferences_searcher = PreferencesSearcher::new(repository);

        let response = get(preferences_searcher).await;

        assert_eq!(response, Ok(all_on_response()));
    }

    #[tokio::test]
    async fn it_should_save_preferences() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let preferences_updater = PreferencesUpdater::new(repository.clone());
        let request = UpdatePreferencesRequest {
            categories_enabled: Some(false),
            ..all_on_request()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(
            response,
            Ok(PreferencesResponse {
                categories_enabled: false,
                ..all_on_response()
            })
        );
        assert_eq!(
            repository.find().unwrap(),
            Some(Preferences {
                categories_enabled: false,
                ..Preferences::default()
            })
        );
    }

    #[tokio::test]
    async fn it_should_replace_previously_saved_preferences() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        repository
            .save(&Preferences {
                categories_enabled: false,
                ..Preferences::default()
            })
            .unwrap();
        let preferences_updater = PreferencesUpdater::new(repository.clone());
        let request = UpdatePreferencesRequest {
            home_show_latest: Some(false),
            ..all_on_request()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(
            response,
            Ok(PreferencesResponse {
                home_show_latest: false,
                ..all_on_response()
            })
        );
        assert_eq!(
            repository.find().unwrap(),
            Some(Preferences {
                home_show_latest: false,
                ..Preferences::default()
            })
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_save_if_a_preference_missing() {
        let request = UpdatePreferencesRequest {
            home_show_latest: None,
            ..all_on_request()
        };

        let response = update(any_preferences_updater(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Whether to show latest videos on home must be stated (missing)"
            ))
        );
    }

    async fn get(
        preferences_searcher: PreferencesSearcher,
    ) -> Result<PreferencesResponse, ApiError> {
        get_preferences(State(preferences_searcher))
            .await
            .map(|Json(response)| response)
    }

    async fn update(
        preferences_updater: PreferencesUpdater,
        request: UpdatePreferencesRequest,
    ) -> Result<PreferencesResponse, ApiError> {
        update_preferences(State(preferences_updater), Json(request))
            .await
            .map(|Json(response)| response)
    }

    /// An updater for tests whose request is rejected before reaching it: its
    /// repository sits on an unmigrated in-memory database, so a request that
    /// wrongly got through would fail loudly instead of passing.
    fn any_preferences_updater() -> PreferencesUpdater {
        PreferencesUpdater::new(Arc::new(SqlitePreferencesRepository::new(
            Database::single(Connection::open_in_memory().unwrap()),
        )))
    }

    fn all_on_request() -> UpdatePreferencesRequest {
        UpdatePreferencesRequest {
            categories_enabled: Some(true),
            home_show_continue_watching: Some(true),
            home_show_categories: Some(true),
            home_show_category_colors: Some(true),
            home_show_quick_watches: Some(true),
            home_show_latest: Some(true),
        }
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
