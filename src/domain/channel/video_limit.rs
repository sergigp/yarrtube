use super::errors::VideoLimitError;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoLimit(u32);

impl VideoLimit {
    pub fn new(value: i64) -> Result<Self, VideoLimitError> {
        if value <= 0 {
            return Err(VideoLimitError(format!(
                "Video limit must be a positive integer (got {value})"
            )));
        }
        Ok(Self(value as u32))
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl fmt::Display for VideoLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_positive_limit() {
        let limit = VideoLimit::new(10).unwrap();
        assert_eq!(limit.value(), 10);
    }

    #[test]
    fn it_should_reject_a_zero_or_negative_limit() {
        assert!(VideoLimit::new(0).is_err());
        assert!(VideoLimit::new(-5).is_err());
    }
}
