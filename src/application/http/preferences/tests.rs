use super::*;
use crate::application::http::test_support::categorization_resetter;
use crate::domain::category::{Category, CategoryColor, CategoryId, CategoryName, CategoryPrompt};
use crate::domain::preferences::{
    CategorizationSettings, Preferences, ResolvedPreferences, SummarySettings,
};
use crate::domain::video::{StepState, Video, VideoId, YoutubeVideoId};
use crate::domain::video_category::{Verdict, VerdictAuthor, VideoCategoryVerdict};
use crate::infrastructure::repositories::sqlite_category_repository::{
    CategoryRepository, SqliteCategoryRepository,
};
use crate::infrastructure::repositories::sqlite_preferences_repository::{
    PreferencesRepository, SqlitePreferencesRepository,
};
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::{
    SqliteVideoCategoryVerdictRepository, VideoCategoryVerdictRepository,
};
use crate::infrastructure::repositories::sqlite_video_repository::{
    SqliteVideoRepository, VideoRepository,
};
use crate::infrastructure::shared::category_lock::CategoryLock;
use crate::infrastructure::shared::sqlite_connection::{Database, TestDatabase};
use crate::infrastructure::shared::system_clock::FixedClock;
use chrono::{DateTime, Duration, Utc};
use rusqlite::Connection;
use std::sync::Arc;

#[tokio::test]
async fn it_should_return_the_defaults_if_nothing_saved() {
    let db = TestDatabase::new();
    let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
    let preferences_searcher = PreferencesSearcher::new(
        repository,
        Arc::new(SqliteCategoryRepository::new(db.database())),
    );

    let response = get(preferences_searcher).await;

    assert_eq!(response, Ok(all_on_response()));
}

#[tokio::test]
async fn it_should_return_an_auto_context_length_from_the_categories_with_a_prompt() {
    let db = TestDatabase::new();
    let category_repository = Arc::new(SqliteCategoryRepository::new(db.database()));
    (1..=9).for_each(|n| {
        category_repository
            .insert(&prompted_category(&format!("Category {n}")))
            .unwrap()
    });
    category_repository
        .insert(&prompted_category("Funny").with_ai_prompt(None))
        .unwrap();
    let preferences_searcher = PreferencesSearcher::new(
        Arc::new(SqlitePreferencesRepository::new(db.database())),
        category_repository,
    );

    let response = get(preferences_searcher).await;

    assert_eq!(
        response,
        Ok(PreferencesResponse {
            categorization_context_length: 16384,
            categorization_context_length_is_auto: true,
            ..all_on_response()
        })
    );
}

#[tokio::test]
async fn it_should_return_4096_for_auto_with_few_categories() {
    let db = TestDatabase::new();
    let category_repository = Arc::new(SqliteCategoryRepository::new(db.database()));
    ["Chess", "Tech"].iter().for_each(|name| {
        category_repository
            .insert(&prompted_category(name))
            .unwrap()
    });
    let preferences_searcher = PreferencesSearcher::new(
        Arc::new(SqlitePreferencesRepository::new(db.database())),
        category_repository,
    );

    let response = get(preferences_searcher).await;

    assert_eq!(
        response,
        Ok(PreferencesResponse {
            categorization_context_length: 4096,
            categorization_context_length_is_auto: true,
            ..all_on_response()
        })
    );
}

#[tokio::test]
async fn it_should_override_the_categorization_context_length() {
    let db = TestDatabase::new();
    let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
    let category_repository = SqliteCategoryRepository::new(db.database());
    (1..=9).for_each(|n| {
        category_repository
            .insert(&prompted_category(&format!("Category {n}")))
            .unwrap()
    });
    let preferences_updater = preferences_updater(&db, repository.clone());
    let request = UpdatePreferencesRequest {
        categorization_context_length: Some(Some(8192)),
        ..UpdatePreferencesRequest::default()
    };

    let response = update(preferences_updater, request).await;

    assert_eq!(
        response,
        Ok(PreferencesResponse {
            categorization_context_length: 8192,
            categorization_context_length_is_auto: false,
            ..all_on_response()
        })
    );
    assert_eq!(
        repository.find().unwrap(),
        Some(Preferences {
            categorization: CategorizationSettings {
                context_length: Some(ContextLength::new(8192).unwrap()),
                ..CategorizationSettings::default()
            },
            ..Preferences::default()
        })
    );
}

#[tokio::test]
async fn it_should_go_back_to_the_auto_context_length() {
    let db = TestDatabase::new();
    let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
    repository
        .save(&Preferences {
            categorization: CategorizationSettings {
                context_length: Some(ContextLength::new(8192).unwrap()),
                ..CategorizationSettings::default()
            },
            ..Preferences::default()
        })
        .unwrap();
    let preferences_updater = preferences_updater(&db, repository.clone());
    let request = UpdatePreferencesRequest {
        categorization_context_length: Some(None),
        ..UpdatePreferencesRequest::default()
    };

    let response = update(preferences_updater, request).await;

    assert_eq!(response, Ok(all_on_response()));
    assert_eq!(repository.find().unwrap(), Some(Preferences::default()));
}

#[tokio::test]
async fn it_should_keep_auto_when_saving_other_preferences() {
    let db = TestDatabase::new();
    let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
    let preferences_updater = preferences_updater(&db, repository.clone());
    let request = UpdatePreferencesRequest {
        categorization_temperature: Some(0.5),
        ..UpdatePreferencesRequest::default()
    };

    update(preferences_updater, request).await.unwrap();

    assert_eq!(
        repository
            .find()
            .unwrap()
            .map(|preferences| preferences.categorization.context_length),
        Some(None)
    );
}

#[tokio::test]
async fn it_should_return_the_ai_defaults_if_nothing_saved() {
    let db = TestDatabase::new();
    let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
    let preferences_searcher = PreferencesSearcher::new(
        repository,
        Arc::new(SqliteCategoryRepository::new(db.database())),
    );

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
async fn it_should_reset_failed_summaries_when_the_summary_settings_change() {
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
        let video = failed_summary_video();
        video_repository.insert(&video).unwrap();
        let preferences_updater = preferences_updater(&db, repository.clone());

        update(preferences_updater, request).await.unwrap();

        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(Video {
                version: 1,
                summary: StepState::pending(),
                updated_at: fixed_timestamp(),
                ..video
            })
        );
    }
}

#[tokio::test]
async fn it_should_keep_failed_summaries_on_an_unrelated_change() {
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
        let video = failed_summary_video();
        video_repository.insert(&video).unwrap();
        let preferences_updater = preferences_updater(&db, repository.clone());

        update(preferences_updater, request).await.unwrap();

        assert_eq!(video_repository.find(&video.id).unwrap(), Some(video));
    }
}

#[tokio::test]
async fn it_should_reset_failed_verdicts_when_the_categorization_model_changes() {
    let requests = [
        UpdatePreferencesRequest {
            ollama_url: Some("http://other.local:11434".to_string()),
            ..UpdatePreferencesRequest::default()
        },
        UpdatePreferencesRequest {
            categorization_model: Some("gemma3:4b".to_string()),
            ..UpdatePreferencesRequest::default()
        },
        UpdatePreferencesRequest {
            categorization_prompt: Some(Some("Answer yes or no.".to_string())),
            ..UpdatePreferencesRequest::default()
        },
    ];

    for request in requests {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let verdict_repository = Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database()));
        let video_repository = SqliteVideoRepository::new(db.database());
        let video = Video {
            id: verdict_video_id(),
            ..Video::create(
                YoutubeVideoId::new("yt1").unwrap(),
                "A video",
                fixed_timestamp(),
            )
            .mark_categorization_failed(fixed_timestamp())
        };
        video_repository.insert(&video).unwrap();
        let decided = [
            verdict("chess", Verdict::Yes),
            verdict("funny", Verdict::No),
        ];
        decided
            .iter()
            .chain(&[verdict("tech", Verdict::Failed)])
            .for_each(|verdict| verdict_repository.save(verdict).unwrap());
        let preferences_updater = PreferencesUpdater::new(
            repository,
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(FixedClock(fixed_timestamp())),
            verdict_repository.clone(),
            Arc::new(CategoryLock::default()),
            Arc::new(SqliteCategoryRepository::new(db.database())),
            categorization_resetter(db.database()),
        );

        update(preferences_updater, request).await.unwrap();

        assert_eq!(
            verdict_repository.find_many(&[verdict_video_id()]).unwrap(),
            decided.to_vec()
        );
        assert_eq!(
            video_repository.find(&video.id).unwrap(),
            Some(Video {
                version: 1,
                categorization: StepState::pending(),
                updated_at: fixed_timestamp(),
                ..video
            })
        );
    }
}

#[tokio::test]
async fn it_should_keep_a_failed_categorization_with_no_failed_verdict_when_the_model_changes() {
    let db = TestDatabase::new();
    let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
    let video_repository = SqliteVideoRepository::new(db.database());
    let video = Video::create(
        YoutubeVideoId::new("yt1").unwrap(),
        "A video",
        fixed_timestamp(),
    )
    .mark_categorization_failed(fixed_timestamp());
    video_repository.insert(&video).unwrap();
    let preferences_updater = preferences_updater(&db, repository);

    update(
        preferences_updater,
        UpdatePreferencesRequest {
            categorization_model: Some("gemma3:4b".to_string()),
            ..UpdatePreferencesRequest::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(video_repository.find(&video.id).unwrap(), Some(video));
}

#[tokio::test]
async fn it_should_keep_failed_verdicts_when_the_same_settings_are_saved() {
    let requests = [
        UpdatePreferencesRequest {
            categorization_model: Some("qwen3:4b".to_string()),
            ..UpdatePreferencesRequest::default()
        },
        UpdatePreferencesRequest {
            categorization_enabled: Some(true),
            ..UpdatePreferencesRequest::default()
        },
        UpdatePreferencesRequest {
            summaries_model: Some("gemma3:4b".to_string()),
            ..UpdatePreferencesRequest::default()
        },
    ];

    for request in requests {
        let db = TestDatabase::new();
        let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let verdict_repository = Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database()));
        let failed = verdict("tech", Verdict::Failed);
        verdict_repository.save(&failed).unwrap();
        let preferences_updater = PreferencesUpdater::new(
            repository,
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(FixedClock(fixed_timestamp())),
            verdict_repository.clone(),
            Arc::new(CategoryLock::default()),
            Arc::new(SqliteCategoryRepository::new(db.database())),
            categorization_resetter(db.database()),
        );

        update(preferences_updater, request).await.unwrap();

        assert_eq!(
            verdict_repository.find_many(&[verdict_video_id()]).unwrap(),
            vec![failed]
        );
    }
}

#[tokio::test]
async fn it_should_return_the_categorization_defaults_if_nothing_saved() {
    let db = TestDatabase::new();
    let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
    let preferences_searcher = PreferencesSearcher::new(
        repository,
        Arc::new(SqliteCategoryRepository::new(db.database())),
    );

    let response = get(preferences_searcher).await;

    assert_eq!(
        response,
        Ok(PreferencesResponse {
            categorization_enabled: false,
            categorization_model: "qwen3:4b".to_string(),
            categorization_prompt: include_str!("../../../../prompts/categorization/v2-batch.md")
                .to_string(),
            categorization_prompt_is_default: true,
            ..all_on_response()
        })
    );
}

#[tokio::test]
async fn it_should_ship_the_categorization_prompt_v2_batch() {
    let db = TestDatabase::new();
    let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
    let preferences_searcher = PreferencesSearcher::new(
        repository,
        Arc::new(SqliteCategoryRepository::new(db.database())),
    );

    let response = get(preferences_searcher).await;

    assert_eq!(
        response.map(|preferences| (
            preferences.categorization_prompt,
            preferences.categorization_prompt_is_default
        )),
        Ok((
            include_str!("../../../../prompts/categorization/v2-batch.md").to_string(),
            true
        ))
    );
}

#[tokio::test]
async fn it_should_return_the_categorization_generation_defaults_if_nothing_saved() {
    let db = TestDatabase::new();
    let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
    let preferences_searcher = PreferencesSearcher::new(
        repository,
        Arc::new(SqliteCategoryRepository::new(db.database())),
    );

    let response = get(preferences_searcher).await;

    assert_eq!(
        response.map(|preferences| (
            preferences.categorization_context_length,
            preferences.categorization_temperature,
            preferences.categorization_thinking
        )),
        Ok((4096, 0.0, true))
    );
}

#[tokio::test]
async fn it_should_save_the_categorization_settings() {
    let db = TestDatabase::new();
    let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
    let preferences_updater = preferences_updater(&db, repository.clone());
    let request = UpdatePreferencesRequest {
        categorization_enabled: Some(true),
        categorization_model: Some("gemma3:4b".to_string()),
        categorization_prompt: Some(Some("Answer yes or no.".to_string())),
        ..UpdatePreferencesRequest::default()
    };

    let response = update(preferences_updater, request).await;

    assert_eq!(
        response,
        Ok(PreferencesResponse {
            categorization_enabled: true,
            categorization_model: "gemma3:4b".to_string(),
            categorization_prompt: "Answer yes or no.".to_string(),
            categorization_prompt_is_default: false,
            ..all_on_response()
        })
    );
    assert_eq!(
        repository.find().unwrap(),
        Some(Preferences {
            categorization: CategorizationSettings {
                enabled: true,
                model: CategorizationModel::new("gemma3:4b").unwrap(),
                prompt: Some(CategorizationPrompt::new("Answer yes or no.").unwrap()),
                ..CategorizationSettings::default()
            },
            ..Preferences::default()
        })
    );
}

#[tokio::test]
async fn it_should_save_the_categorization_generation_settings() {
    let db = TestDatabase::new();
    let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
    let preferences_updater = preferences_updater(&db, repository.clone());
    let request = UpdatePreferencesRequest {
        categorization_context_length: Some(Some(8192)),
        categorization_temperature: Some(0.5),
        categorization_thinking: Some(false),
        ..UpdatePreferencesRequest::default()
    };

    let response = update(preferences_updater, request).await;

    assert_eq!(
        response,
        Ok(PreferencesResponse {
            categorization_enabled: false,
            categorization_context_length: 8192,
            categorization_context_length_is_auto: false,
            categorization_temperature: 0.5,
            categorization_thinking: false,
            ..all_on_response()
        })
    );
    assert_eq!(
        repository.find().unwrap(),
        Some(Preferences {
            categorization: CategorizationSettings {
                context_length: Some(ContextLength::new(8192).unwrap()),
                temperature: Temperature::new(0.5).unwrap(),
                thinking: false,
                ..CategorizationSettings::default()
            },
            ..Preferences::default()
        })
    );
}

#[tokio::test]
async fn it_should_fail_if_the_categorization_context_length_is_out_of_range() {
    let request = UpdatePreferencesRequest {
        categorization_context_length: Some(Some(512)),
        ..UpdatePreferencesRequest::default()
    };

    let response = update(any_preferences_updater(), request).await;

    assert_eq!(
        response,
        Err(ApiError::bad_request(
            "Context length must be between 1024 and 131072 (got 512)"
        ))
    );
}

#[tokio::test]
async fn it_should_fail_if_the_categorization_temperature_is_out_of_range() {
    let request = UpdatePreferencesRequest {
        categorization_temperature: Some(2.5),
        ..UpdatePreferencesRequest::default()
    };

    let response = update(any_preferences_updater(), request).await;

    assert_eq!(
        response,
        Err(ApiError::bad_request(
            "Temperature must be between 0 and 2 (got 2.5)"
        ))
    );
}

#[tokio::test]
async fn it_should_go_back_to_the_shipped_categorization_prompt() {
    let db = TestDatabase::new();
    let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
    let saved = Preferences {
        categorization: CategorizationSettings {
            prompt: Some(CategorizationPrompt::new("Answer yes or no.").unwrap()),
            ..CategorizationSettings::default()
        },
        ..Preferences::default()
    };
    repository.save(&saved).unwrap();
    let preferences_updater = preferences_updater(&db, repository.clone());
    let request = UpdatePreferencesRequest {
        categorization_prompt: Some(None),
        ..UpdatePreferencesRequest::default()
    };

    let response = update(preferences_updater, request).await;

    assert_eq!(response, Ok(all_on_response()));
    assert_eq!(repository.find().unwrap(), Some(Preferences::default()));
}

#[tokio::test]
async fn it_should_fail_if_the_categorization_model_is_blank() {
    let request = UpdatePreferencesRequest {
        categorization_model: Some("  ".to_string()),
        ..UpdatePreferencesRequest::default()
    };

    let response = update(any_preferences_updater(), request).await;

    assert_eq!(
        response,
        Err(ApiError::bad_request(
            "Categorization model must be named (got \"  \")"
        ))
    );
}

#[tokio::test]
async fn it_should_fail_if_the_categorization_prompt_is_blank() {
    let request = UpdatePreferencesRequest {
        categorization_prompt: Some(Some("  ".to_string())),
        ..UpdatePreferencesRequest::default()
    };

    let response = update(any_preferences_updater(), request).await;

    assert_eq!(
        response,
        Err(ApiError::bad_request(
            "Categorization prompt must not be empty"
        ))
    );
}

#[tokio::test]
async fn it_should_keep_failed_summaries_when_the_categorization_settings_change() {
    let db = TestDatabase::new();
    let repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
    let video_repository = SqliteVideoRepository::new(db.database());
    let video = failed_summary_video();
    video_repository.insert(&video).unwrap();
    let preferences_updater = preferences_updater(&db, repository.clone());
    let request = UpdatePreferencesRequest {
        categorization_model: Some("gemma3:4b".to_string()),
        ..UpdatePreferencesRequest::default()
    };

    update(preferences_updater, request).await.unwrap();

    assert_eq!(video_repository.find(&video.id).unwrap(), Some(video));
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
    let preferences_searcher = PreferencesSearcher::new(
        repository,
        Arc::new(SqliteCategoryRepository::new(db.database())),
    );

    let response = get(preferences_searcher).await;

    assert_eq!(
        response,
        Ok(PreferencesResponse {
            home_show_quick_watches: false,
            ..all_on_response()
        })
    );
}

async fn get(preferences_searcher: PreferencesSearcher) -> Result<PreferencesResponse, ApiError> {
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
        Arc::new(SqliteVideoCategoryVerdictRepository::new(database())),
        Arc::new(CategoryLock::default()),
        Arc::new(SqliteCategoryRepository::new(database())),
        categorization_resetter(database()),
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
        Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        Arc::new(CategoryLock::default()),
        Arc::new(SqliteCategoryRepository::new(db.database())),
        categorization_resetter(db.database()),
    )
}

fn fixed_timestamp() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
}

fn failed_summary_video() -> Video {
    let created = fixed_timestamp() - Duration::days(1);
    Video::create(YoutubeVideoId::new("yt1").unwrap(), "A video", created)
        .mark_summary_failed(created)
}

/// The shipped prompt with one rule added to its `system` section.
fn verdict_video_id() -> VideoId {
    VideoId::new("rec1").unwrap()
}

/// The AI's `verdict` for `verdict_video_id()` and `category`.
fn verdict(category: &str, verdict: Verdict) -> VideoCategoryVerdict {
    VideoCategoryVerdict {
        video_id: verdict_video_id(),
        category_id: CategoryId::new(category).unwrap(),
        verdict,
        decided_by: VerdictAuthor::Ai,
        model: Some("qwen3:4b".to_string()),
        reason: None,
        decided_at: fixed_timestamp(),
    }
}

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

/// A category named `name` with an AI prompt.
fn prompted_category(name: &str) -> Category {
    Category::create(
        CategoryName::new(name).unwrap(),
        CategoryColor::new(CategoryColor::PALETTE[0]).unwrap(),
        fixed_timestamp(),
    )
    .with_ai_prompt(Some(
        CategoryPrompt::new(format!("The video is about {name}."))
            .unwrap()
            .unwrap(),
    ))
}

fn all_on_response() -> PreferencesResponse {
    PreferencesResponse {
        categories_enabled: true,
        home_show_continue_watching: true,
        home_show_categories: true,
        home_show_quick_watches: true,
        home_show_latest: true,
        theme: "light".to_string(),
        ..PreferencesResponse::from(ResolvedPreferences {
            preferences: Preferences::default(),
            categorization_context_length: ContextLength::auto(0),
        })
    }
}
