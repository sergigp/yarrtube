use crate::domain::shared::ValidationError;

/// How long a video lasts, in whole seconds, as reported by the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoDuration(i64);

impl VideoDuration {
    pub fn new(seconds: i64) -> Result<Self, ValidationError> {
        Ok(Self(seconds))
    }

    pub fn seconds(&self) -> i64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_positive_duration() {
        assert_eq!(VideoDuration::new(1), Ok(VideoDuration(1)));
        assert_eq!(VideoDuration::new(120), Ok(VideoDuration(120)));
    }
}
