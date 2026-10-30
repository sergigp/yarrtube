use crate::domain::preferences::Preferences;
use crate::infrastructure::shared::sqlite_connection::Database;

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
        let _ = &self.db;
        Ok(None)
    }

    fn save(&self, _preferences: &Preferences) -> anyhow::Result<()> {
        Ok(())
    }
}
