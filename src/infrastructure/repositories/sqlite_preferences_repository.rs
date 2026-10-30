use crate::domain::preferences::{Preferences, Theme};
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
}
