use crate::domain::preferences::{
    Preferences, PreferencesChange, ResolvedPreferences, UpdatePreferencesError,
};
use crate::domain::services::{
    CategorizationResetter, CategorizationResetterApi, PlexLabelSyncer, PlexLabelSyncerApi,
};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
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
    categorization_resetter: CategorizationResetter,
    clock: Arc<dyn Clock>,
    video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
    category_lock: Arc<CategoryLock>,
    category_repository: Arc<dyn CategoryRepository>,
    anthropic_available: bool,
    plex_label_syncer: Option<PlexLabelSyncer>,
}

impl PreferencesUpdater {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repository: Arc<dyn PreferencesRepository>,
        video_repository: Arc<dyn VideoRepository>,
        clock: Arc<dyn Clock>,
        video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
        category_lock: Arc<CategoryLock>,
        category_repository: Arc<dyn CategoryRepository>,
        categorization_resetter: CategorizationResetter,
        anthropic_available: bool,
        plex_label_syncer: Option<PlexLabelSyncer>,
    ) -> Self {
        Self {
            repository,
            categorization_resetter,
            video_repository,
            clock,
            video_category_verdict_repository,
            category_lock,
            category_repository,
            anthropic_available,
            plex_label_syncer,
        }
    }
}

pub trait PreferencesUpdaterApi: Send + Sync {
    /// Applies `change` to the stored preferences (or the defaults) and
    /// saves the result, or saves nothing when any of it is invalid. New
    /// summary settings may succeed where the old ones failed, so they put
    /// every Unavailable summary back to Missing; new categorization settings
    /// delete every unavailable verdict, so those pairs are pending again.
    /// Returns them with the categorization context length they resolve to.
    fn update(
        &self,
        change: PreferencesChange,
    ) -> Result<ResolvedPreferences, UpdatePreferencesError>;
}

impl PreferencesUpdaterApi for PreferencesUpdater {
    fn update(
        &self,
        change: PreferencesChange,
    ) -> Result<ResolvedPreferences, UpdatePreferencesError> {
        let stored = self.stored()?;
        let updated = stored
            .clone()
            .apply(change, self.anthropic_available)
            .map_err(UpdatePreferencesError::Invalid)?;
        self.repository
            .save(&updated)
            .map_err(UpdatePreferencesError::Repository)?;
        if stored.changes_summaries(&updated) {
            self.video_repository
                .reset_failed_summaries(self.clock.now())
                .map_err(UpdatePreferencesError::Repository)?;
        }
        if stored.changes_categorization(&updated) {
            self.reset_failed_categorizations()?;
        }
        if stored.changes_plex_playlists(&updated) {
            self.request_label_sync()?;
        }
        self.resolve(updated)
    }
}

impl PreferencesUpdater {
    /// `preferences` with the categorization context length they resolve
    /// to for the categories with an AI prompt now.
    fn resolve(
        &self,
        preferences: Preferences,
    ) -> Result<ResolvedPreferences, UpdatePreferencesError> {
        let categories = self
            .category_repository
            .list()
            .map_err(UpdatePreferencesError::Repository)?;
        Ok(ResolvedPreferences::new(
            preferences,
            &categories,
            self.anthropic_available,
        ))
    }

    /// Category playlists are replaced by a label sync that runs now, when
    /// Plex is configured.
    fn request_label_sync(&self) -> Result<(), UpdatePreferencesError> {
        self.plex_label_syncer
            .as_ref()
            .map(|syncer| syncer.request_sync(self.clock.now()))
            .unwrap_or(Ok(()))
            .map_err(UpdatePreferencesError::Repository)
    }

    fn stored(&self) -> Result<Preferences, UpdatePreferencesError> {
        self.repository
            .find()
            .map(Option::unwrap_or_default)
            .map_err(UpdatePreferencesError::Repository)
    }

    /// New settings may get answers the old ones couldn't: every `failed`
    /// verdict goes and the categorization of each video that had one is
    /// Pending again. Under the category lock, so a running categorization
    /// giving up either sees the new settings or has its verdicts deleted
    /// here.
    fn reset_failed_categorizations(&self) -> Result<(), UpdatePreferencesError> {
        let _guard = self.category_lock.lock();
        self.video_category_verdict_repository
            .delete_failed()
            .and_then(|video_ids| self.categorization_resetter.reset(&video_ids))
            .map_err(UpdatePreferencesError::Repository)
    }
}
