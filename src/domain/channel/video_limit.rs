use crate::domain::shared::ValidationError;
use std::fmt;

const MAX_VIDEO_LIMIT: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoLimit(u32);

impl VideoLimit {
    pub fn new(value: i64) -> Result<Self, ValidationError> {
        u32::try_from(value)
            .ok()
            .filter(|limit| (1..=MAX_VIDEO_LIMIT).contains(limit))
            .map(Self)
            .ok_or_else(|| {
                ValidationError(format!(
                    "Video limit must be between 1 and {MAX_VIDEO_LIMIT} (got {value})"
                ))
            })
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
        assert_eq!(VideoLimit::new(10), Ok(VideoLimit(10)));
    }

    #[test]
    fn it_should_accept_a_limit_of_one() {
        assert_eq!(VideoLimit::new(1), Ok(VideoLimit(1)));
    }

    #[test]
    fn it_should_accept_the_maximum_limit() {
        assert_eq!(VideoLimit::new(1000), Ok(VideoLimit(1000)));
    }

    #[test]
    fn it_should_reject_a_zero_limit() {
        assert_eq!(
            VideoLimit::new(0),
            Err(ValidationError(
                "Video limit must be between 1 and 1000 (got 0)".to_string()
            ))
        );
    }

    #[test]
    fn it_should_reject_a_negative_limit() {
        assert_eq!(
            VideoLimit::new(-5),
            Err(ValidationError(
                "Video limit must be between 1 and 1000 (got -5)".to_string()
            ))
        );
    }

    #[test]
    fn it_should_reject_a_limit_above_the_maximum() {
        assert_eq!(
            VideoLimit::new(1001),
            Err(ValidationError(
                "Video limit must be between 1 and 1000 (got 1001)".to_string()
            ))
        );
    }

    #[test]
    fn it_should_reject_a_limit_that_would_wrap_to_zero_in_32_bits() {
        assert_eq!(
            VideoLimit::new(4_294_967_296),
            Err(ValidationError(
                "Video limit must be between 1 and 1000 (got 4294967296)".to_string()
            ))
        );
    }

    #[test]
    fn it_should_reject_a_limit_that_would_wrap_to_a_valid_limit_in_32_bits() {
        assert_eq!(
            VideoLimit::new(4_294_967_306),
            Err(ValidationError(
                "Video limit must be between 1 and 1000 (got 4294967306)".to_string()
            ))
        );
    }
}
