use crate::domain::shared::ValidationError;
use std::fmt;

/// Where every AI feature runs: a user-hosted Ollama server, or Anthropic's
/// API with the key from the environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[expect(
    dead_code,
    reason = "walking skeleton: constructed once parsing is implemented"
)]
pub enum AiProvider {
    #[default]
    Ollama,
    Anthropic,
}

impl AiProvider {
    pub fn new(_value: &str) -> Result<Self, ValidationError> {
        Ok(Self::default())
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::Anthropic => "anthropic",
        }
    }
}

impl fmt::Display for AiProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
