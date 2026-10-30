use crate::domain::preferences::Preferences;
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

/// The stored JSON document.
#[derive(Serialize, Deserialize)]
struct StoredPreferences {
    categories_enabled: bool,
    home_show_continue_watching: bool,
    home_show_categories: bool,
    home_show_category_colors: bool,
    home_show_quick_watches: bool,
    home_show_latest: bool,
}

impl From<&Preferences> for StoredPreferences {
    fn from(preferences: &Preferences) -> Self {
        Self {
            categories_enabled: preferences.categories_enabled,
            home_show_continue_watching: preferences.home_show_continue_watching,
            home_show_categories: preferences.home_show_categories,
            home_show_category_colors: preferences.home_show_category_colors,
            home_show_quick_watches: preferences.home_show_quick_watches,
            home_show_latest: preferences.home_show_latest,
        }
    }
}

impl From<StoredPreferences> for Preferences {
    fn from(stored: StoredPreferences) -> Self {
        Self {
            categories_enabled: stored.categories_enabled,
            home_show_continue_watching: stored.home_show_continue_watching,
            home_show_categories: stored.home_show_categories,
            home_show_category_colors: stored.home_show_category_colors,
            home_show_quick_watches: stored.home_show_quick_watches,
            home_show_latest: stored.home_show_latest,
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
            home_show_category_colors: false,
            home_show_quick_watches: true,
            home_show_latest: false,
        };

        repository.save(&preferences).unwrap();

        assert_eq!(repository.find().unwrap(), Some(preferences));
    }
}
