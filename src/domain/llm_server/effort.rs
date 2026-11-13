use crate::domain::shared::ValidationError;
use std::fmt;

/// How much an Anthropic model reasons before answering: more is slower and
/// costs more tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Effort {
    Low,
    #[default]
    Medium,
    High,
}

impl Effort {
    pub fn new(value: &str) -> Result<Self, ValidationError> {
        match value {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(ValidationError(format!(
                "Effort must be \"low\", \"medium\" or \"high\" (got \"{value}\")"
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl fmt::Display for Effort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_parse_each_effort() {
        assert_eq!(
            ["low", "medium", "high"].map(Effort::new),
            [Ok(Effort::Low), Ok(Effort::Medium), Ok(Effort::High)]
        );
    }

    #[test]
    fn it_should_reject_an_unknown_effort() {
        assert_eq!(Effort::new("max"), Err(rejected("max")));
    }

    #[test]
    fn it_should_reject_an_uppercase_effort() {
        assert_eq!(Effort::new("High"), Err(rejected("High")));
    }

    fn rejected(value: &str) -> ValidationError {
        ValidationError(format!(
            "Effort must be \"low\", \"medium\" or \"high\" (got \"{value}\")"
        ))
    }
}
