use super::channel_episode_number::ChannelEpisodeNumber;
use super::playlist_episode_number::PlaylistEpisodeNumber;
use super::video_output_entry::SEASON_FOLDER_PREFIX;
use crate::domain::shared::ValidationError;

/// A video's TV-show episode number within its show, by that show's rule: a
/// channel's from the publish date, a playlist's from playlist order.
/// Assigned once and never changed, so Plex watch state (keyed by show +
/// season + episode) stays on the right video.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EpisodeNumber {
    Channel(ChannelEpisodeNumber),
    Playlist(PlaylistEpisodeNumber),
}

impl EpisodeNumber {
    /// Decodes the stored `season`/`episode` columns: season `1` is a
    /// playlist number, any other season a channel's publish year.
    pub fn from_columns(season: i32, episode: u32) -> Result<Self, ValidationError> {
        match season {
            PlaylistEpisodeNumber::SEASON => Ok(PlaylistEpisodeNumber::new(episode)?.into()),
            _ => Ok(ChannelEpisodeNumber::new(season, episode)?.into()),
        }
    }

    pub fn season(&self) -> i32 {
        match self {
            Self::Channel(number) => number.season(),
            Self::Playlist(_) => PlaylistEpisodeNumber::SEASON,
        }
    }

    pub fn episode(&self) -> u32 {
        match self {
            Self::Channel(number) => number.episode(),
            Self::Playlist(number) => number.episode(),
        }
    }

    /// The `S<season>E<episode>` code, e.g. `S2026E031501` or `S01E007`.
    pub fn code(&self) -> String {
        match self {
            Self::Channel(number) => format!("S{}E{:06}", number.season(), number.episode()),
            Self::Playlist(number) => {
                format!(
                    "S{:02}E{:03}",
                    PlaylistEpisodeNumber::SEASON,
                    number.episode()
                )
            }
        }
    }

    /// The season folder name, e.g. `Season 2026` or `Season 1`.
    pub fn season_folder(&self) -> String {
        format!("{SEASON_FOLDER_PREFIX}{}", self.season())
    }
}

impl From<ChannelEpisodeNumber> for EpisodeNumber {
    fn from(number: ChannelEpisodeNumber) -> Self {
        Self::Channel(number)
    }
}

impl From<PlaylistEpisodeNumber> for EpisodeNumber {
    fn from(number: PlaylistEpisodeNumber) -> Self {
        Self::Playlist(number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_render_a_playlist_code_padded_to_three_digits() {
        let number = EpisodeNumber::from(PlaylistEpisodeNumber::new(7).unwrap());

        assert_eq!(
            (number.code(), number.season_folder()),
            ("S01E007".to_string(), "Season 1".to_string())
        );
    }

    #[test]
    fn it_should_widen_a_playlist_code_past_999() {
        let number = EpisodeNumber::from(PlaylistEpisodeNumber::new(1000).unwrap());

        assert_eq!(number.code(), "S01E1000");
    }

    #[test]
    fn it_should_decode_season_one_as_a_playlist_number() {
        assert_eq!(
            EpisodeNumber::from_columns(1, 7),
            Ok(EpisodeNumber::Playlist(
                PlaylistEpisodeNumber::new(7).unwrap()
            ))
        );
        assert_eq!(
            EpisodeNumber::from_columns(2026, 31501),
            Ok(EpisodeNumber::Channel(
                ChannelEpisodeNumber::new(2026, 31501).unwrap()
            ))
        );
    }
}
