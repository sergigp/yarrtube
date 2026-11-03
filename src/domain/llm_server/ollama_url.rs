use crate::domain::shared::ValidationError;
use std::fmt;

/// Where an Ollama server listens: an absolute http(s) URL, kept without a
/// trailing slash so paths can be appended to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OllamaUrl(String);

impl OllamaUrl {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        match ["http://", "https://"]
            .iter()
            .any(|scheme| value.starts_with(scheme))
        {
            true => Ok(Self(value.trim_end_matches('/').to_string())),
            false => Err(ValidationError(format!(
                "Ollama URL must be an http(s) URL (got \"{value}\")"
            ))),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OllamaUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
