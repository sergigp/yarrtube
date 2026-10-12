use crate::domain::shared::ValidationError;
use chrono::{DateTime, Datelike, Utc};

/// A channel video's episode number: the season is the publish year and the
/// episode is the publish month and day followed by a two-digit same-day
/// index (`MMDD * 100 + index`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChannelEpisodeNumber {
    season: i32,
    episode: u32,
}

impl ChannelEpisodeNumber {
    pub fn new(season: i32, episode: u32) -> Result<Self, ValidationError> {
        Ok(Self { season, episode })
    }

    /// The day's first episode (index `01`) for a video published at
    /// `published_at`.
    pub fn first_of_day(published_at: DateTime<Utc>) -> Self {
        Self {
            season: published_at.year(),
            episode: (published_at.month() * 100 + published_at.day()) * 100 + 1,
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
        self.episode / 100
    }
}
