use crate::domain::preferences::{Preferences, ReadPreferencesError};
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
use std::sync::Arc;

/// Reads the user's preferences.
#[derive(Clone)]
pub struct PreferencesSearcher {
    repository: Arc<dyn PreferencesRepository>,
}

impl PreferencesSearcher {
    pub fn new(repository: Arc<dyn PreferencesRepository>) -> Self {
        Self { repository }
    }
}

pub trait PreferencesSearcherApi: Send + Sync {
    /// The saved preferences, or the defaults when none were saved.
    fn get(&self) -> Result<Preferences, ReadPreferencesError>;
}

impl PreferencesSearcherApi for PreferencesSearcher {
    fn get(&self) -> Result<Preferences, ReadPreferencesError> {
        self.repository
            .find()
            .map(Option::unwrap_or_default)
            .map_err(ReadPreferencesError::Repository)
    }
}
