use crate::domain::llm_server::OllamaUrl;
use crate::domain::preferences::{
    ContextLength, Preferences, SummaryModel, SummaryPrompt, SummarySettings, Temperature, Theme,
};
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

pub trait PreferencesRepository: Send + Sync {
    fn find(&self) -> anyhow::Result<Option<Preferences>>;
    /// Inserts or replaces the single row.
    fn save(&self, preferences: &Preferences) -> anyhow::Result<()>;
}

pub struct SqlitePreferencesRepository {
    db: Database,
}

impl SqlitePreferencesRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl PreferencesRepository for SqlitePreferencesRepository {
    fn find(&self) -> anyhow::Result<Option<Preferences>> {
        let conn = self.db.read()?;
        conn.query_row("SELECT data FROM preferences WHERE id = 1", [], |row| {
            row.get::<_, String>(0)
        })
        .optional()
        .inspect_err(|e| tracing::error!(error = %e, "failed to query preferences"))
        .context("failed to query preferences")?
        .map(|data| Self::data_to_preferences(&data))
        .transpose()
    }

    fn save(&self, preferences: &Preferences) -> anyhow::Result<()> {
        let data = serde_json::to_string(&StoredPreferences::from(preferences))
            .context("failed to serialize preferences")?;
        let conn = self.db.write()?;
        conn.execute(
            "INSERT INTO preferences (id, data) VALUES (1, ?1)
             ON CONFLICT (id) DO UPDATE SET data = excluded.data",
            params![data],
        )
        .inspect_err(|e| tracing::error!(error = %e, "failed to save preferences"))
        .context("failed to save preferences")?;
        Ok(())
    }
}

impl SqlitePreferencesRepository {
    fn data_to_preferences(data: &str) -> anyhow::Result<Preferences> {
        serde_json::from_str::<StoredPreferences>(data)
            .inspect_err(|e| tracing::error!(error = %e, "failed to parse stored preferences"))
            .context("failed to parse stored preferences")
            .map(Preferences::from)
    }
}

/// The stored JSON document. Keys missing from it, e.g. a preference added
/// after it was saved, read as their default.
#[derive(Serialize, Deserialize)]
struct StoredPreferences {
    categories_enabled: Option<bool>,
    home_show_continue_watching: Option<bool>,
    home_show_categories: Option<bool>,
    home_show_quick_watches: Option<bool>,
    home_show_latest: Option<bool>,
    theme: Option<String>,
    ai_enabled: Option<bool>,
    ollama_url: Option<String>,
    summaries_enabled: Option<bool>,
    summaries_model: Option<String>,
    summaries_context_length: Option<i64>,
    summaries_temperature: Option<f64>,
    summaries_thinking: Option<bool>,
    /// Only once the user saved one: missing reads as the shipped prompt.
    summaries_prompt: Option<String>,
}

impl From<&Preferences> for StoredPreferences {
    fn from(preferences: &Preferences) -> Self {
        Self {
            categories_enabled: Some(preferences.categories_enabled),
            home_show_continue_watching: Some(preferences.home_show_continue_watching),
            home_show_categories: Some(preferences.home_show_categories),
            home_show_quick_watches: Some(preferences.home_show_quick_watches),
            home_show_latest: Some(preferences.home_show_latest),
            theme: Some(preferences.theme.as_str().to_string()),
            ai_enabled: Some(preferences.ai_enabled),
            ollama_url: preferences.ollama_url.as_ref().map(|url| url.to_string()),
            summaries_enabled: Some(preferences.summaries.enabled),
            summaries_model: Some(preferences.summaries.model.as_str().to_string()),
            summaries_context_length: Some(preferences.summaries.context_length.value().into()),
            summaries_temperature: Some(preferences.summaries.temperature.value()),
            summaries_thinking: Some(preferences.summaries.thinking),
            summaries_prompt: preferences
                .summaries
                .prompt
                .as_ref()
                .map(|prompt| prompt.as_str().to_string()),
        }
    }
}

impl From<StoredPreferences> for Preferences {
    fn from(stored: StoredPreferences) -> Self {
        let defaults = Preferences::default();
        Self {
            categories_enabled: stored
                .categories_enabled
                .unwrap_or(defaults.categories_enabled),
            home_show_continue_watching: stored
                .home_show_continue_watching
                .unwrap_or(defaults.home_show_continue_watching),
            home_show_categories: stored
                .home_show_categories
                .unwrap_or(defaults.home_show_categories),
            home_show_quick_watches: stored
                .home_show_quick_watches
                .unwrap_or(defaults.home_show_quick_watches),
            home_show_latest: stored.home_show_latest.unwrap_or(defaults.home_show_latest),
            // An unknown stored theme reads as the default rather than
            // failing every read of the preferences.
            theme: stored
                .theme
                .and_then(|theme| Theme::new(theme).ok())
                .unwrap_or(defaults.theme),
            ai_enabled: stored.ai_enabled.unwrap_or(defaults.ai_enabled),
            ollama_url: stored.ollama_url.and_then(|url| OllamaUrl::new(url).ok()),
            // Like the theme, a stored value no longer valid reads as the
            // default.
            summaries: SummarySettings {
                enabled: stored
                    .summaries_enabled
                    .unwrap_or(defaults.summaries.enabled),
                model: stored
                    .summaries_model
                    .and_then(|model| SummaryModel::new(model).ok())
                    .unwrap_or(defaults.summaries.model),
                context_length: stored
                    .summaries_context_length
                    .and_then(|length| ContextLength::new(length).ok())
                    .unwrap_or(defaults.summaries.context_length),
                temperature: stored
                    .summaries_temperature
                    .and_then(|temperature| Temperature::new(temperature).ok())
                    .unwrap_or(defaults.summaries.temperature),
                thinking: stored
                    .summaries_thinking
                    .unwrap_or(defaults.summaries.thinking),
                // Kept as saved, even when this version's rules reject it:
                // reading it as the default would erase it on the next save.
                prompt: stored.summaries_prompt.map(SummaryPrompt::from_stored),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;

    #[test]
    fn it_should_find_nothing_if_never_saved() {
        let db = TestDatabase::new();
        let repository = SqlitePreferencesRepository::new(db.database());

        assert_eq!(repository.find().unwrap(), None);
    }

    #[test]
    fn it_should_find_saved_preferences() {
        let db = TestDatabase::new();
        let repository = SqlitePreferencesRepository::new(db.database());
        let preferences = Preferences {
            categories_enabled: false,
            home_show_continue_watching: false,
            home_show_categories: true,
            home_show_quick_watches: true,
            home_show_latest: false,
            theme: Theme::Dark,
            ..Preferences::default()
        };

        repository.save(&preferences).unwrap();

        assert_eq!(repository.find().unwrap(), Some(preferences));
    }

    #[test]
    fn it_should_read_missing_keys_as_defaults() {
        let db = TestDatabase::new();
        db.connection()
            .execute(
                "INSERT INTO preferences (id, data) VALUES (1, '{\"categories_enabled\": false}')",
                [],
            )
            .unwrap();
        let repository = SqlitePreferencesRepository::new(db.database());

        assert_eq!(
            repository.find().map_err(|e| e.to_string()),
            Ok(Some(Preferences {
                categories_enabled: false,
                ..Preferences::default()
            }))
        );
    }

    #[test]
    fn it_should_find_saved_ai_preferences() {
        let db = TestDatabase::new();
        let repository = SqlitePreferencesRepository::new(db.database());
        let preferences = Preferences {
            ai_enabled: true,
            ollama_url: Some(OllamaUrl::new("http://mac.local:11434").unwrap()),
            summaries: SummarySettings {
                enabled: true,
                model: SummaryModel::new("gemma3:4b").unwrap(),
                context_length: ContextLength::new(8192).unwrap(),
                temperature: Temperature::new(0.7).unwrap(),
                thinking: false,
                prompt: Some(SummaryPrompt::shipped()),
            },
            ..Preferences::default()
        };

        repository.save(&preferences).unwrap();

        assert_eq!(repository.find().unwrap(), Some(preferences));
    }

    #[test]
    fn it_should_read_missing_ai_keys_as_defaults() {
        let db = TestDatabase::new();
        db.connection()
            .execute(
                "INSERT INTO preferences (id, data) VALUES (1, '{\"categories_enabled\": true, \"home_show_continue_watching\": true, \"home_show_categories\": true, \"home_show_quick_watches\": false, \"home_show_latest\": true, \"theme\": \"dark\"}')",
                [],
            )
            .unwrap();
        let repository = SqlitePreferencesRepository::new(db.database());

        assert_eq!(
            repository.find().map_err(|e| e.to_string()),
            Ok(Some(Preferences {
                home_show_quick_watches: false,
                theme: Theme::Dark,
                ..Preferences::default()
            }))
        );
    }

    #[test]
    fn it_should_read_invalid_ai_values_as_defaults() {
        let db = TestDatabase::new();
        db.connection()
            .execute(
                "INSERT INTO preferences (id, data) VALUES (1, '{\"ollama_url\": \"nowhere\", \"summaries_model\": \"\", \"summaries_context_length\": 5, \"summaries_temperature\": 9.0}')",
                [],
            )
            .unwrap();
        let repository = SqlitePreferencesRepository::new(db.database());

        assert_eq!(
            repository.find().map_err(|e| e.to_string()),
            Ok(Some(Preferences::default()))
        );
    }

    #[test]
    fn it_should_find_no_prompt_if_saved_without_one() {
        let db = TestDatabase::new();
        let repository = SqlitePreferencesRepository::new(db.database());
        repository
            .save(&Preferences {
                summaries: SummarySettings {
                    prompt: Some(SummaryPrompt::shipped()),
                    ..SummarySettings::default()
                },
                ..Preferences::default()
            })
            .unwrap();

        repository.save(&Preferences::default()).unwrap();

        assert_eq!(repository.find().unwrap(), Some(Preferences::default()));
    }

    #[test]
    fn it_should_keep_a_stored_prompt_the_rules_reject() {
        let db = TestDatabase::new();
        db.connection()
            .execute(
                "INSERT INTO preferences (id, data) VALUES (1, '{\"summaries_prompt\": \"## system\"}')",
                [],
            )
            .unwrap();
        let repository = SqlitePreferencesRepository::new(db.database());

        assert_eq!(
            repository.find().map_err(|e| e.to_string()),
            Ok(Some(Preferences {
                summaries: SummarySettings {
                    prompt: Some(SummaryPrompt::from_stored("## system")),
                    ..SummarySettings::default()
                },
                ..Preferences::default()
            }))
        );
    }
}
