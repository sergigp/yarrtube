use crate::domain::preferences::{Preferences, PreferencesChange, UpdatePreferencesError};
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
use std::sync::Arc;

/// Changes the user's preferences.
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
    /// Applies `change` to the stored preferences (or the defaults) and
    /// saves the result, or saves nothing when any of it is invalid.
    fn update(&self, change: PreferencesChange) -> Result<Preferences, UpdatePreferencesError>;
}

impl PreferencesUpdaterApi for PreferencesUpdater {
    fn update(&self, change: PreferencesChange) -> Result<Preferences, UpdatePreferencesError> {
        let updated = self
            .stored()?
            .apply(change)
            .map_err(UpdatePreferencesError::Invalid)?;
        self.repository
            .save(&updated)
            .map_err(UpdatePreferencesError::Repository)?;
        Ok(updated)
    }
}

impl PreferencesUpdater {
    fn stored(&self) -> Result<Preferences, UpdatePreferencesError> {
        self.repository
            .find()
            .map(Option::unwrap_or_default)
            .map_err(UpdatePreferencesError::Repository)
    }
}
