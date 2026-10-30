use crate::domain::preferences::Preferences;

/// How many videos each home section holds, and how many category sections
/// there are at most.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomeLimits {
    pub continue_watching: usize,
    pub category_sections: usize,
    pub category_videos: usize,
    pub quick_watches: usize,
    pub latest: usize,
}

impl HomeLimits {
    /// 0 for every section the preferences turn off.
    pub fn following(self, preferences: &Preferences) -> Self {
        Self {
            continue_watching: Self::unless_off(
                self.continue_watching,
                preferences.home_show_continue_watching,
            ),
            category_sections: Self::unless_off(
                self.category_sections,
                preferences.shows_home_categories(),
            ),
            quick_watches: Self::unless_off(
                self.quick_watches,
                preferences.home_show_quick_watches,
            ),
            latest: Self::unless_off(self.latest, preferences.home_show_latest),
            ..self
        }
    }

    fn unless_off(limit: usize, shown: bool) -> usize {
        if shown { limit } else { 0 }
    }
}
