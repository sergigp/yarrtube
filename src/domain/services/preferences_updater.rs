use crate::domain::preferences::{Preferences, PreferencesChange, UpdatePreferencesError};
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::VideoCategoryVerdictRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::category_lock::CategoryLock;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Changes the user's preferences.
#[derive(Clone)]
pub struct PreferencesUpdater {
    repository: Arc<dyn PreferencesRepository>,
    video_repository: Arc<dyn VideoRepository>,
    clock: Arc<dyn Clock>,
    video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
    category_lock: Arc<CategoryLock>,
}

impl PreferencesUpdater {
    pub fn new(
        repository: Arc<dyn PreferencesRepository>,
        video_repository: Arc<dyn VideoRepository>,
        clock: Arc<dyn Clock>,
        video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
        category_lock: Arc<CategoryLock>,
    ) -> Self {
        Self {
            repository,
            video_repository,
            clock,
            video_category_verdict_repository,
            category_lock,
        }
    }
}

pub trait PreferencesUpdaterApi: Send + Sync {
    /// Applies `change` to the stored preferences (or the defaults) and
    /// saves the result, or saves nothing when any of it is invalid. New
    /// summary settings may succeed where the old ones failed, so they put
    /// every Unavailable summary back to Missing; new categorization settings
    /// delete every unavailable verdict, so those pairs are pending again.
    fn update(&self, change: PreferencesChange) -> Result<Preferences, UpdatePreferencesError>;
}

impl PreferencesUpdaterApi for PreferencesUpdater {
    fn update(&self, change: PreferencesChange) -> Result<Preferences, UpdatePreferencesError> {
        let stored = self.stored()?;
        let updated = stored
            .clone()
            .apply(change)
            .map_err(UpdatePreferencesError::Invalid)?;
        self.repository
            .save(&updated)
            .map_err(UpdatePreferencesError::Repository)?;
        if stored.changes_summaries(&updated) {
            self.video_repository
                .reset_unavailable_summaries(self.clock.now())
                .map_err(UpdatePreferencesError::Repository)?;
        }
        if stored.changes_categorization(&updated) {
            self.reset_unavailable_verdicts()?;
        }
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

    /// New settings may get answers the old ones couldn't: every unavailable
    /// verdict goes. Under the category lock, so a running categorization
    /// giving up either sees the new settings or has its verdicts deleted here.
    fn reset_unavailable_verdicts(&self) -> Result<(), UpdatePreferencesError> {
        let _guard = self.category_lock.lock();
        self.video_category_verdict_repository
            .delete_unavailable()
            .map_err(UpdatePreferencesError::Repository)
    }
}
