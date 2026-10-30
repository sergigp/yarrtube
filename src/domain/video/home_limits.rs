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
    pub fn following(self, _preferences: &Preferences) -> Self {
        self
    }
}
