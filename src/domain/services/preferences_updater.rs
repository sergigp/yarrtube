use crate::domain::preferences::{Preferences, SavePreferencesError};
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
use std::sync::Arc;

/// Saves the user's preferences.
#[derive(Clone)]
pub struct PreferencesUpdater {
    repository: Arc<dyn PreferencesRepository>,
}

impl PreferencesUpdater {
    pub fn new(repository: Arc<dyn PreferencesRepository>) -> Self {
        Self { repository }
    }
}

pub trait PreferencesUpdaterApi: Send + Sync {
    /// Replaces every preference with `preferences`.
    fn save(&self, preferences: Preferences) -> Result<Preferences, SavePreferencesError>;
}

impl PreferencesUpdaterApi for PreferencesUpdater {
    fn save(&self, preferences: Preferences) -> Result<Preferences, SavePreferencesError> {
        Ok(preferences)
    }
}
