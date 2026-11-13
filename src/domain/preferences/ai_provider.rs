use crate::domain::shared::ValidationError;
use std::fmt;

/// Where every AI feature runs: a user-hosted Ollama server, or Anthropic's
/// API with the key from the environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AiProvider {
    #[default]
    Ollama,
    Anthropic,
}

impl AiProvider {
    pub fn new(value: &str) -> Result<Self, ValidationError> {
        match value {
            "ollama" => Ok(Self::Ollama),
            "anthropic" => Ok(Self::Anthropic),
            _ => Err(ValidationError(format!(
                "AI provider must be \"ollama\" or \"anthropic\" (got \"{value}\")"
            ))),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_parse_ollama() {
        assert_eq!(AiProvider::new("ollama"), Ok(AiProvider::Ollama));
    }

    #[test]
    fn it_should_parse_anthropic() {
        assert_eq!(AiProvider::new("anthropic"), Ok(AiProvider::Anthropic));
    }

    #[test]
    fn it_should_reject_an_unknown_provider() {
        assert_eq!(AiProvider::new("Anthropic"), Err(rejected("Anthropic")));
    }

    #[test]
    fn it_should_reject_an_empty_provider() {
        assert_eq!(AiProvider::new(""), Err(rejected("")));
    }

    fn rejected(value: &str) -> ValidationError {
        ValidationError(format!(
            "AI provider must be \"ollama\" or \"anthropic\" (got \"{value}\")"
        ))
    }
}
