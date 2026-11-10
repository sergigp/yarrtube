use crate::domain::preferences::{Preferences, ReadPreferencesError, ResolvedPreferences};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
use std::sync::Arc;

/// Reads the user's preferences.
#[derive(Clone)]
pub struct PreferencesSearcher {
    repository: Arc<dyn PreferencesRepository>,
    category_repository: Arc<dyn CategoryRepository>,
}

impl PreferencesSearcher {
    pub fn new(
        repository: Arc<dyn PreferencesRepository>,
        category_repository: Arc<dyn CategoryRepository>,
    ) -> Self {
        Self {
            repository,
            category_repository,
        }
    }
}

pub trait PreferencesSearcherApi: Send + Sync {
    /// The saved preferences, or the defaults when none were saved, with
    /// the categorization context length they resolve to now.
    fn get(&self) -> Result<ResolvedPreferences, ReadPreferencesError>;
}

impl PreferencesSearcherApi for PreferencesSearcher {
    fn get(&self) -> Result<ResolvedPreferences, ReadPreferencesError> {
        let preferences = self
            .repository
            .find()
            .map(Option::unwrap_or_default)
            .map_err(ReadPreferencesError::Repository)?;
        self.resolve(preferences)
    }
}

impl PreferencesSearcher {
    fn resolve(
        &self,
        preferences: Preferences,
    ) -> Result<ResolvedPreferences, ReadPreferencesError> {
        let _ = &self.category_repository;
        Ok(ResolvedPreferences {
            categorization_context_length: preferences.categorization.effective_context_length(0),
            preferences,
        })
    }
}
