use crate::domain::shared::ValidationError;

/// A playlist video's episode number: its place in the playlist's numbering,
/// assigned from the playlist's counter when the video is first stored. The
/// season is always `1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlaylistEpisodeNumber {
    episode: u32,
}

impl PlaylistEpisodeNumber {
    pub const SEASON: i32 = 1;

    pub fn new(episode: u32) -> Result<Self, ValidationError> {
        if episode == 0 {
            return Err(ValidationError(format!(
                "Playlist episode number must be at least 1 (got {episode})"
            )));
        }
        Ok(Self { episode })
    }

    pub fn episode(&self) -> u32 {
        self.episode
    }

    /// The number after this one.
    pub fn next(&self) -> Result<Self, ValidationError> {
        Self::new(self.episode + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_reject_episode_zero() {
        assert_eq!(
            PlaylistEpisodeNumber::new(0),
            Err(ValidationError(
                "Playlist episode number must be at least 1 (got 0)".to_string()
            ))
        );
        assert_eq!(
            PlaylistEpisodeNumber::new(1),
            Ok(PlaylistEpisodeNumber { episode: 1 })
        );
    }

    #[test]
    fn it_should_give_the_next_number() {
        assert_eq!(
            PlaylistEpisodeNumber::new(6).unwrap().next(),
            PlaylistEpisodeNumber::new(7)
        );
    }
}
