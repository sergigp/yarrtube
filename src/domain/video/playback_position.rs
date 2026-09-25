use crate::domain::shared::ValidationError;

/// How far into a video playback got, in whole seconds from the start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlaybackPosition(i64);

impl PlaybackPosition {
    pub fn new(seconds: i64) -> Result<Self, ValidationError> {
        Ok(Self(seconds))
    }

    /// The beginning of the video.
    pub fn start() -> Self {
        Self(0)
    }

    pub fn seconds(&self) -> i64 {
        self.0
    }
}
