use super::Theme;

/// The user's daemon-wide display preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preferences {
    pub categories_enabled: bool,
    pub home_show_continue_watching: bool,
    pub home_show_categories: bool,
    pub home_show_quick_watches: bool,
    pub home_show_latest: bool,
    pub theme: Theme,
}

impl Default for Preferences {
    /// Everything on, in the light theme.
    fn default() -> Self {
        Self {
            categories_enabled: true,
            home_show_continue_watching: true,
            home_show_categories: true,
            home_show_quick_watches: true,
            home_show_latest: true,
            theme: Theme::default(),
        }
    }
}

impl Preferences {
    /// Category sections on home need categories on as well.
    pub fn shows_home_categories(&self) -> bool {
        self.categories_enabled && self.home_show_categories
    }
}
