use crate::domain::preferences::Preferences;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    #[serde(default)]
    pub categories_enabled: Option<bool>,
    #[serde(default)]
    pub home_show_continue_watching: Option<bool>,
    #[serde(default)]
    pub home_show_categories: Option<bool>,
    #[serde(default)]
    pub home_show_quick_watches: Option<bool>,
    #[serde(default)]
    pub home_show_latest: Option<bool>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PreferencesResponse {
    pub categories_enabled: bool,
    pub home_show_continue_watching: bool,
    pub home_show_categories: bool,
    pub home_show_quick_watches: bool,
    pub home_show_latest: bool,
}

impl From<Preferences> for PreferencesResponse {
    fn from(preferences: Preferences) -> Self {
        Self {
            categories_enabled: preferences.categories_enabled,
            home_show_continue_watching: preferences.home_show_continue_watching,
            home_show_categories: preferences.home_show_categories,
            home_show_quick_watches: preferences.home_show_quick_watches,
            home_show_latest: preferences.home_show_latest,
        }
    }
}
