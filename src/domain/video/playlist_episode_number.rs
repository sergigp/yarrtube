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
        Ok(Self { episode })
    }

    pub fn episode(&self) -> u32 {
        self.episode
    }

    /// The number after this one.
    pub fn next(&self) -> Result<Self, ValidationError> {
        Ok(*self)
    }
}
