use crate::domain::shared::ValidationError;

/// How far past the start playback must have got for a video to count as
/// started rather than just opened.
const STARTED_MIN_SECONDS: i64 = 30;

/// The share of a video's known duration, in percent, playback must reach
/// to count as started.
const STARTED_MIN_SHARE_PERCENT: i64 = 20;

/// How far into a video of known duration playback counts as started
/// whatever its share, so a long video is not held back for long.
const STARTED_ENOUGH_SECONDS: i64 = 600;

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

    /// Past the first 30 seconds and, with a known positive
    /// `duration_seconds`, at least 20% of it or 10 minutes: started, not
    /// just opened or sampled.
    pub fn is_started(&self, duration_seconds: Option<i64>) -> bool {
        self.0 > STARTED_MIN_SECONDS
            && duration_seconds
                .filter(|duration| *duration > 0)
                .is_none_or(|duration| self.reaches_started_share_of(duration))
    }

    /// At least 20% of `duration_seconds`, or 10 minutes.
    fn reaches_started_share_of(&self, duration_seconds: i64) -> bool {
        self.0 * 100 >= duration_seconds * STARTED_MIN_SHARE_PERCENT
            || self.0 >= STARTED_ENOUGH_SECONDS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_count_a_position_as_started_only_past_30_seconds() {
        let started: Vec<bool> = [0, 30, 31]
            .into_iter()
            .map(|seconds| PlaybackPosition::new(seconds).unwrap().is_started(None))
            .collect();

        assert_eq!(started, vec![false, false, true]);
    }

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
