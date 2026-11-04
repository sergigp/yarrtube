pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::llm_server::OllamaUrl;
use crate::domain::preferences::{
    ContextLength, PreferencesChange, SummaryModel, SummaryPrompt, Temperature, Theme,
    UpdatePreferencesError,
};
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
    Json(request): Json<UpdatePreferencesRequest>,
) -> Result<Json<PreferencesResponse>, ApiError> {
    let change = preferences_change(request)?;
    let updated = run_blocking(move || preferences_updater.update(change))
        .await?
        .map_err(update_preferences_error)?;
    Ok(Json(PreferencesResponse::from(updated)))
}

/// The request's fields as value objects, rejecting the first invalid one.
fn preferences_change(request: UpdatePreferencesRequest) -> Result<PreferencesChange, ApiError> {
    Ok(PreferencesChange {
        categories_enabled: request.categories_enabled,
        home_show_continue_watching: request.home_show_continue_watching,
        home_show_categories: request.home_show_categories,
        home_show_quick_watches: request.home_show_quick_watches,
        home_show_latest: request.home_show_latest,
        theme: request.theme.map(Theme::new).transpose()?,
        ai_enabled: request.ai_enabled,
        ollama_url: request.ollama_url.map(ollama_url).transpose()?,
        summaries_enabled: request.summaries_enabled,
        summaries_model: request.summaries_model.map(SummaryModel::new).transpose()?,
        summaries_context_length: request
            .summaries_context_length
            .map(ContextLength::new)
            .transpose()?,
        summaries_temperature: request
            .summaries_temperature
            .map(Temperature::new)
            .transpose()?,
        summaries_thinking: request.summaries_thinking,
        summaries_prompt: request
            .summaries_prompt
            .map(|prompt| prompt.map(SummaryPrompt::new).transpose())
            .transpose()?,
    })
}

/// An empty URL unsets it.
fn ollama_url(value: String) -> Result<Option<OllamaUrl>, ApiError> {
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(OllamaUrl::new(value)?))
    }
}

fn update_preferences_error(error: UpdatePreferencesError) -> ApiError {
    match error {
        UpdatePreferencesError::Invalid(e) => ApiError::from(e),
        e @ UpdatePreferencesError::Repository(_) => ApiError::internal(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::preferences::{Preferences, SummarySettings};
    use crate::domain::video::{SummaryState, Video, VideoId};
    use crate::infrastructure::repositories::sqlite_preferences_repository::{
        PreferencesRepository, SqlitePreferencesRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::{Database, TestDatabase};
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Duration, Utc};
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
    async fn it_should_return_the_ai_defaults_if_nothing_saved() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let preferences_searcher = PreferencesSearcher::new(repository);

        let response = get(preferences_searcher).await;

        assert_eq!(
            response,
            Ok(PreferencesResponse {
                ai_enabled: false,
                ollama_url: None,
                summaries_enabled: false,
                summaries_model: "qwen3:4b".to_string(),
                summaries_context_length: 12288,
                summaries_temperature: 0.3,
                summaries_thinking: true,
                summaries_prompt: include_str!("../../../../prompts/summaries/v3.md").to_string(),
                summaries_prompt_is_default: true,
                ..all_on_response()
            })
        );
    }

    #[tokio::test]
    async fn it_should_reset_unavailable_summaries_when_the_summary_settings_change() {
        let requests = [
            UpdatePreferencesRequest {
                ollama_url: Some("http://other.local:11434".to_string()),
                ..UpdatePreferencesRequest::default()
            },
            UpdatePreferencesRequest {
                summaries_model: Some("gemma3:4b".to_string()),
                ..UpdatePreferencesRequest::default()
            },
            UpdatePreferencesRequest {
                summaries_context_length: Some(16384),
                ..UpdatePreferencesRequest::default()
            },
            UpdatePreferencesRequest {
                summaries_temperature: Some(0.5),
                ..UpdatePreferencesRequest::default()
            },
            UpdatePreferencesRequest {
                summaries_thinking: Some(false),
                ..UpdatePreferencesRequest::default()
            },
            UpdatePreferencesRequest {
                summaries_prompt: Some(Some(edited_prompt())),
                ..UpdatePreferencesRequest::default()
            },
        ];

        for request in requests {
            let db = TestDatabase::new();
            let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
            let video_repository = SqliteVideoRepository::new(db.database());
            let video = unavailable_video();
            video_repository.save(&video).unwrap();
            let preferences_updater = preferences_updater(&db, repository.clone());

            update(preferences_updater, request).await.unwrap();

            assert_eq!(
                video_repository.find(&video.id).unwrap(),
                Some(Video {
                    summary_state: SummaryState::Missing,
                    updated_at: fixed_timestamp(),
                    ..video
                })
            );
        }
    }

    #[tokio::test]
    async fn it_should_keep_unavailable_summaries_on_an_unrelated_change() {
        let requests = [
            UpdatePreferencesRequest {
                categories_enabled: Some(false),
                ..UpdatePreferencesRequest::default()
            },
            UpdatePreferencesRequest {
                summaries_enabled: Some(true),
                ..UpdatePreferencesRequest::default()
            },
            UpdatePreferencesRequest {
                summaries_context_length: Some(12288),
                ..UpdatePreferencesRequest::default()
            },
        ];

        for request in requests {
            let db = TestDatabase::new();
            let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
            let video_repository = SqliteVideoRepository::new(db.database());
            let video = unavailable_video();
            video_repository.save(&video).unwrap();
            let preferences_updater = preferences_updater(&db, repository.clone());

            update(preferences_updater, request).await.unwrap();

            assert_eq!(video_repository.find(&video.id).unwrap(), Some(video));
        }
    }

    #[tokio::test]
    async fn it_should_save_preferences() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let preferences_updater = preferences_updater(&db, repository.clone());
        let request = UpdatePreferencesRequest {
            categories_enabled: Some(false),
            ..UpdatePreferencesRequest::default()
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
    async fn it_should_change_only_the_preferences_sent() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        repository
            .save(&Preferences {
                home_show_latest: false,
                ..Preferences::default()
            })
            .unwrap();
        let preferences_updater = preferences_updater(&db, repository.clone());
        let request = UpdatePreferencesRequest {
            categories_enabled: Some(false),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(
            response,
            Ok(PreferencesResponse {
                categories_enabled: false,
                home_show_latest: false,
                ..all_on_response()
            })
        );
        assert_eq!(
            repository.find().unwrap(),
            Some(Preferences {
                categories_enabled: false,
                home_show_latest: false,
                ..Preferences::default()
            })
        );
    }

    #[tokio::test]
    async fn it_should_keep_everything_on_an_empty_patch() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        repository
            .save(&Preferences {
                theme: Theme::Dark,
                ..Preferences::default()
            })
            .unwrap();
        let preferences_updater = preferences_updater(&db, repository.clone());

        let response = update(preferences_updater, UpdatePreferencesRequest::default()).await;

        assert_eq!(
            response,
            Ok(PreferencesResponse {
                theme: "dark".to_string(),
                ..all_on_response()
            })
        );
        assert_eq!(
            repository.find().unwrap(),
            Some(Preferences {
                theme: Theme::Dark,
                ..Preferences::default()
            })
        );
    }

    #[tokio::test]
    async fn it_should_save_the_ollama_url_without_trailing_slash() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let preferences_updater = preferences_updater(&db, repository.clone());
        let request = UpdatePreferencesRequest {
            ollama_url: Some("http://mac.local:11434/".to_string()),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(
            response,
            Ok(PreferencesResponse {
                ollama_url: Some("http://mac.local:11434".to_string()),
                ..all_on_response()
            })
        );
        assert_eq!(
            repository.find().unwrap(),
            Some(Preferences {
                ollama_url: Some(OllamaUrl::new("http://mac.local:11434").unwrap()),
                ..Preferences::default()
            })
        );
    }

    #[tokio::test]
    async fn it_should_unset_the_ollama_url_on_an_empty_string() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        repository
            .save(&Preferences {
                ollama_url: Some(OllamaUrl::new("http://mac.local:11434").unwrap()),
                ..Preferences::default()
            })
            .unwrap();
        let preferences_updater = preferences_updater(&db, repository.clone());
        let request = UpdatePreferencesRequest {
            ollama_url: Some(String::new()),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(response, Ok(all_on_response()));
        assert_eq!(repository.find().unwrap(), Some(Preferences::default()));
    }

    #[tokio::test]
    async fn it_should_enable_ai_with_a_url() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let preferences_updater = preferences_updater(&db, repository.clone());
        let request = UpdatePreferencesRequest {
            ai_enabled: Some(true),
            ollama_url: Some("http://mac.local:11434".to_string()),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(
            response,
            Ok(PreferencesResponse {
                ai_enabled: true,
                ollama_url: Some("http://mac.local:11434".to_string()),
                ..all_on_response()
            })
        );
        assert_eq!(
            repository.find().unwrap(),
            Some(Preferences {
                ai_enabled: true,
                ollama_url: Some(OllamaUrl::new("http://mac.local:11434").unwrap()),
                ..Preferences::default()
            })
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_enable_ai_without_a_url() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let preferences_updater = preferences_updater(&db, repository.clone());
        let request = UpdatePreferencesRequest {
            ai_enabled: Some(true),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("Ollama URL must be set to enable AI"))
        );
        assert_eq!(repository.find().unwrap(), None);
    }

    #[tokio::test]
    async fn it_should_fail_to_unset_the_url_while_ai_is_on() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let saved = Preferences {
            ai_enabled: true,
            ollama_url: Some(OllamaUrl::new("http://mac.local:11434").unwrap()),
            ..Preferences::default()
        };
        repository.save(&saved).unwrap();
        let preferences_updater = preferences_updater(&db, repository.clone());
        let request = UpdatePreferencesRequest {
            ollama_url: Some(String::new()),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("Ollama URL must be set to enable AI"))
        );
        assert_eq!(repository.find().unwrap(), Some(saved));
    }

    #[tokio::test]
    async fn it_should_save_the_summary_settings() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let preferences_updater = preferences_updater(&db, repository.clone());
        let request = UpdatePreferencesRequest {
            summaries_enabled: Some(true),
            summaries_model: Some("gemma3:4b".to_string()),
            summaries_context_length: Some(8192),
            summaries_temperature: Some(0.0),
            summaries_thinking: Some(false),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(
            response,
            Ok(PreferencesResponse {
                summaries_enabled: true,
                summaries_model: "gemma3:4b".to_string(),
                summaries_context_length: 8192,
                summaries_temperature: 0.0,
                summaries_thinking: false,
                ..all_on_response()
            })
        );
        assert_eq!(
            repository.find().unwrap(),
            Some(Preferences {
                summaries: SummarySettings {
                    enabled: true,
                    model: SummaryModel::new("gemma3:4b").unwrap(),
                    context_length: ContextLength::new(8192).unwrap(),
                    temperature: Temperature::new(0.0).unwrap(),
                    thinking: false,
                    prompt: None,
                },
                ..Preferences::default()
            })
        );
    }

    #[tokio::test]
    async fn it_should_fail_if_temperature_out_of_range() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let preferences_updater = preferences_updater(&db, repository.clone());
        let request = UpdatePreferencesRequest {
            categories_enabled: Some(false),
            summaries_temperature: Some(5.0),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Temperature must be between 0 and 2 (got 5)"
            ))
        );
        assert_eq!(repository.find().unwrap(), None);
    }

    #[tokio::test]
    async fn it_should_save_an_edited_prompt() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let preferences_updater = preferences_updater(&db, repository.clone());
        let prompt = edited_prompt();
        let request = UpdatePreferencesRequest {
            summaries_prompt: Some(Some(prompt.clone())),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(
            response,
            Ok(PreferencesResponse {
                summaries_prompt: prompt.clone(),
                summaries_prompt_is_default: false,
                ..all_on_response()
            })
        );
        assert_eq!(
            repository.find().unwrap(),
            Some(Preferences {
                summaries: SummarySettings {
                    prompt: Some(SummaryPrompt::new(prompt).unwrap()),
                    ..SummarySettings::default()
                },
                ..Preferences::default()
            })
        );
    }

    #[tokio::test]
    async fn it_should_not_store_the_prompt_when_saving_other_preferences() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let preferences_updater = preferences_updater(&db, repository.clone());
        let request = UpdatePreferencesRequest {
            summaries_enabled: Some(true),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(
            response,
            Ok(PreferencesResponse {
                summaries_enabled: true,
                ..all_on_response()
            })
        );
        assert_eq!(
            repository.find().unwrap(),
            Some(Preferences {
                summaries: SummarySettings {
                    enabled: true,
                    prompt: None,
                    ..SummarySettings::default()
                },
                ..Preferences::default()
            })
        );
    }

    #[tokio::test]
    async fn it_should_reset_the_prompt_to_the_shipped_one() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        repository
            .save(&Preferences {
                summaries: SummarySettings {
                    prompt: Some(SummaryPrompt::new(edited_prompt()).unwrap()),
                    ..SummarySettings::default()
                },
                ..Preferences::default()
            })
            .unwrap();
        let preferences_updater = preferences_updater(&db, repository.clone());
        let request: UpdatePreferencesRequest =
            serde_json::from_str(r#"{"summaries_prompt": null}"#).unwrap();

        let response = update(preferences_updater, request).await;

        assert_eq!(response, Ok(all_on_response()));
        assert_eq!(repository.find().unwrap(), Some(Preferences::default()));
    }

    #[tokio::test]
    async fn it_should_fail_on_an_invalid_prompt() {
        let request = UpdatePreferencesRequest {
            summaries_prompt: Some(Some(without_map_user_section())),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(any_preferences_updater(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Prompt is missing the \"map user\" section"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_keep_a_saved_prompt_the_current_rules_reject() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let saved = Preferences {
            summaries: SummarySettings {
                prompt: Some(SummaryPrompt::from_stored("## system\nmy own words")),
                ..SummarySettings::default()
            },
            ..Preferences::default()
        };
        repository.save(&saved).unwrap();
        let preferences_updater = preferences_updater(&db, repository.clone());
        let request = UpdatePreferencesRequest {
            theme: Some("dark".to_string()),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(
            response,
            Ok(PreferencesResponse {
                theme: "dark".to_string(),
                summaries_prompt: "## system\nmy own words".to_string(),
                summaries_prompt_is_default: false,
                ..all_on_response()
            })
        );
        assert_eq!(
            repository.find().unwrap(),
            Some(Preferences {
                theme: Theme::Dark,
                ..saved
            })
        );
    }

    #[tokio::test]
    async fn it_should_save_the_theme() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let preferences_updater = preferences_updater(&db, repository.clone());
        let request = UpdatePreferencesRequest {
            theme: Some("dark".to_string()),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(preferences_updater, request).await;

        assert_eq!(
            response,
            Ok(PreferencesResponse {
                theme: "dark".to_string(),
                ..all_on_response()
            })
        );
        assert_eq!(
            repository.find().unwrap(),
            Some(Preferences {
                theme: Theme::Dark,
                ..Preferences::default()
            })
        );
    }

    #[tokio::test]
    async fn it_should_fail_on_an_unknown_theme() {
        let request = UpdatePreferencesRequest {
            theme: Some("sepia".to_string()),
            ..UpdatePreferencesRequest::default()
        };

        let response = update(any_preferences_updater(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Theme must be \"light\" or \"dark\" (got \"sepia\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_saved_preferences() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        repository
            .save(&Preferences {
                home_show_quick_watches: false,
                ..Preferences::default()
            })
            .unwrap();
        let preferences_searcher = PreferencesSearcher::new(repository);

        let response = get(preferences_searcher).await;

        assert_eq!(
            response,
            Ok(PreferencesResponse {
                home_show_quick_watches: false,
                ..all_on_response()
            })
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
        let database = || Database::single(Connection::open_in_memory().unwrap());
        PreferencesUpdater::new(
            Arc::new(SqlitePreferencesRepository::new(database())),
            Arc::new(SqliteVideoRepository::new(database())),
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    fn preferences_updater(
        db: &TestDatabase,
        repository: Arc<SqlitePreferencesRepository>,
    ) -> PreferencesUpdater {
        PreferencesUpdater::new(
            repository,
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn unavailable_video() -> Video {
        let created = fixed_timestamp() - Duration::days(1);
        Video::create(VideoId::new("yt1").unwrap(), "A video", created)
            .mark_summary_unavailable(created)
    }

    /// The shipped prompt with one rule added to its `system` section.
    fn edited_prompt() -> String {
        SummaryPrompt::shipped().as_str().replace(
            "- Ignore sponsors, ads and requests to like or subscribe.\n\n## user",
            "- Ignore sponsors, ads and requests to like or subscribe.\n- Be brief.\n\n## user",
        )
    }

    fn without_map_user_section() -> String {
        let shipped = SummaryPrompt::shipped().as_str().to_string();
        let start = shipped.find("## map user").unwrap();
        let end = shipped.find("## map chapters").unwrap();
        format!("{}{}", &shipped[..start], &shipped[end..])
    }

    fn all_on_response() -> PreferencesResponse {
        PreferencesResponse {
            categories_enabled: true,
            home_show_continue_watching: true,
            home_show_categories: true,
            home_show_quick_watches: true,
            home_show_latest: true,
            theme: "light".to_string(),
            ..PreferencesResponse::from(Preferences::default())
        }
    }
}
