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
            _ => Ok(Self::default()),
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
