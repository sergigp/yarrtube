use crate::domain::shared::ValidationError;
use chrono::{DateTime, Utc};

/// A video's TV-show episode number within its show: the season is the
/// publish year and the episode is the publish month and day followed by a
/// two-digit same-day index (`MMDD * 100 + index`). Assigned once and never
/// changed, so Plex watch state (keyed by show + season + episode) stays on
/// the right video.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EpisodeNumber {
    season: i32,
    episode: u32,
}

impl EpisodeNumber {
    pub fn new(season: i32, episode: u32) -> Result<Self, ValidationError> {
        Ok(Self { season, episode })
    }

    /// The day's first episode (index `01`) for a video published at
    /// `published_at`.
    pub fn first_of_day(_published_at: DateTime<Utc>) -> Self {
        Self {
            season: 0,
            episode: 0,
        }
    }

    pub fn season(&self) -> i32 {
        self.season
    }

    pub fn episode(&self) -> u32 {
        self.episode
    }

    /// The publish month and day (`MMDD`) shared by every episode of a day.
    pub fn same_day_prefix(&self) -> u32 {
        0
    }

    /// The `S<season>E<MMDDii>` code, e.g. `S2026E031501`.
    pub fn code(&self) -> String {
        String::new()
    }

    /// The season folder name, e.g. `Season 2026`.
    pub fn season_folder(&self) -> String {
        String::new()
    }
}
