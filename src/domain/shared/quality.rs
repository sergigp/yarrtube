use super::errors::ValidationError;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    High,
    Mid,
    Low,
}

impl Quality {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        match value.as_str() {
            "high" => Ok(Self::High),
            "mid" => Ok(Self::Mid),
            "low" => Ok(Self::Low),
            _ => Err(ValidationError(format!(
                "Quality must be one of \"high\", \"mid\", or \"low\" (got \"{value}\")"
            ))),
        }
    }

    /// The height in pixels that settles a tier's downloaded quality (see
    /// the `late-video-data` capability). `high` downloads stay uncapped:
    /// 1080 is only where its checks stop.
    pub fn target_height(&self) -> u32 {
        match self {
            Self::High => 1080,
            Self::Mid => 720,
            Self::Low => 480,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Mid => "mid",
            Self::Low => "low",
        }
    }
}

impl fmt::Display for Quality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_parse_high() {
        assert_eq!(Quality::new("high"), Ok(Quality::High));
    }

    #[test]
    fn it_should_parse_mid() {
        assert_eq!(Quality::new("mid"), Ok(Quality::Mid));
    }

    #[test]
    fn it_should_parse_low() {
        assert_eq!(Quality::new("low"), Ok(Quality::Low));
    }

    #[test]
    fn it_should_reject_an_unknown_quality() {
        assert_eq!(
            Quality::new("ultra"),
            Err(ValidationError(
                "Quality must be one of \"high\", \"mid\", or \"low\" (got \"ultra\")".to_string()
            ))
        );
    }

    #[test]
    fn it_should_reject_a_quality_in_a_different_case() {
        assert_eq!(
            Quality::new("HIGH"),
            Err(ValidationError(
                "Quality must be one of \"high\", \"mid\", or \"low\" (got \"HIGH\")".to_string()
            ))
        );
    }

    #[test]
    fn it_should_reject_an_empty_quality() {
        assert_eq!(
            Quality::new(""),
            Err(ValidationError(
                "Quality must be one of \"high\", \"mid\", or \"low\" (got \"\")".to_string()
            ))
        );
    }
}
