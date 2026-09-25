use crate::domain::shared::ValidationError;

/// How far into a video playback got, in whole seconds from the start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlaybackPosition(i64);

impl PlaybackPosition {
    pub fn new(seconds: i64) -> Result<Self, ValidationError> {
        if seconds < 0 {
            return Err(ValidationError(format!(
                "Playback position must not be negative (got {seconds})"
            )));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_zero_and_positive_positions() {
        assert_eq!(PlaybackPosition::new(0), Ok(PlaybackPosition(0)));
        assert_eq!(PlaybackPosition::new(120), Ok(PlaybackPosition(120)));
    }

    #[test]
    fn it_should_reject_a_negative_position() {
        assert_eq!(
            PlaybackPosition::new(-1),
            Err(ValidationError(
                "Playback position must not be negative (got -1)".to_string()
            ))
        );
    }
}
