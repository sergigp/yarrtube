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
    /// A four-digit publish year, and an episode whose `MMDD` is a calendar
    /// month and day and whose same-day index is `01`..=`99`: a 100th video
    /// on one day would spill into the next day's prefix, so it fails instead.
    pub fn new(season: i32, episode: u32) -> Result<Self, ValidationError> {
        if !(1000..=9999).contains(&season) {
            return Err(ValidationError(format!(
                "Channel episode season must be a four-digit year (got {season})"
            )));
        }
        let (month, day, index) = (episode / 10_000, episode / 100 % 100, episode % 100);
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) || index == 0 {
            return Err(ValidationError(format!(
                "Channel episode must be MMDD followed by a same-day index from 01 to 99 (got {episode})"
            )));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_day_and_index_within_range() {
        assert_eq!(
            [
                ChannelEpisodeNumber::new(2026, 10101),
                ChannelEpisodeNumber::new(2026, 123199),
            ],
            [
                Ok(ChannelEpisodeNumber {
                    season: 2026,
                    episode: 10101
                }),
                Ok(ChannelEpisodeNumber {
                    season: 2026,
                    episode: 123199
                }),
            ]
        );
    }

    #[test]
    fn it_should_fail_if_the_season_is_not_a_four_digit_year() {
        assert_eq!(
            [
                ChannelEpisodeNumber::new(1, 31501),
                ChannelEpisodeNumber::new(10000, 31501),
            ],
            [
                Err(ValidationError(
                    "Channel episode season must be a four-digit year (got 1)".to_string()
                )),
                Err(ValidationError(
                    "Channel episode season must be a four-digit year (got 10000)".to_string()
                )),
            ]
        );
    }

    #[test]
    fn it_should_fail_if_the_episode_is_not_a_day_and_index() {
        let invalid = |episode: u32| {
            Err(ValidationError(format!(
                "Channel episode must be MMDD followed by a same-day index from 01 to 99 (got {episode})"
            )))
        };

        assert_eq!(
            [0, 1501, 131501, 33201, 31500].map(|episode| ChannelEpisodeNumber::new(2026, episode)),
            [0, 1501, 131501, 33201, 31500].map(invalid)
        );
    }

    #[test]
    fn it_should_fail_to_number_past_the_99th_video_of_a_day() {
        let last = ChannelEpisodeNumber::new(2026, 31599).unwrap();

        assert_eq!(
            ChannelEpisodeNumber::new(last.season(), last.episode() + 1),
            Err(ValidationError(
                "Channel episode must be MMDD followed by a same-day index from 01 to 99 (got 31600)"
                    .to_string()
            ))
        );
    }

    #[test]
    fn it_should_start_the_day_at_index_one() {
        let published_at = DateTime::parse_from_rfc3339("2026-03-15T10:00:00Z")
            .unwrap()
            .to_utc();

        assert_eq!(
            ChannelEpisodeNumber::first_of_day(published_at),
            ChannelEpisodeNumber::new(2026, 31501).unwrap()
        );
    }
}
