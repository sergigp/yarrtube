/// The user's daemon-wide display preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preferences {
    pub categories_enabled: bool,
    pub home_show_continue_watching: bool,
    pub home_show_categories: bool,
    pub home_show_category_colors: bool,
    pub home_show_quick_watches: bool,
    pub home_show_latest: bool,
}

impl Default for Preferences {
    /// Everything on.
    fn default() -> Self {
        Self {
            categories_enabled: true,
            home_show_continue_watching: true,
            home_show_categories: true,
            home_show_category_colors: true,
            home_show_quick_watches: true,
            home_show_latest: true,
        }
    }
}

impl Preferences {
    /// Category sections on home need categories on as well.
    pub fn shows_home_categories(&self) -> bool {
        self.categories_enabled
    }
}
