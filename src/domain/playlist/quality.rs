use super::errors::PlaylistError;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    High,
    Mid,
    Low,
}

impl Quality {
    pub fn new(value: impl Into<String>) -> Result<Self, PlaylistError> {
        let value = value.into();
        match value.as_str() {
            "high" => Ok(Self::High),
            "mid" => Ok(Self::Mid),
            "low" => Ok(Self::Low),
            _ => Err(PlaylistError(format!(
                "Quality must be one of \"high\", \"mid\", or \"low\" (got \"{value}\")"
            ))),
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
    fn it_should_parse_each_valid_quality_value() {
        assert_eq!(Quality::new("high").unwrap(), Quality::High);
        assert_eq!(Quality::new("mid").unwrap(), Quality::Mid);
        assert_eq!(Quality::new("low").unwrap(), Quality::Low);
    }

    #[test]
    fn it_should_reject_an_invalid_quality_value_with_a_meaningful_message() {
        let error = Quality::new("ultra").unwrap_err();

        assert_eq!(
            error.to_string(),
            "Quality must be one of \"high\", \"mid\", or \"low\" (got \"ultra\")"
        );
    }
}
