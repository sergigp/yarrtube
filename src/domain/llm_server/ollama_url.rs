use crate::domain::shared::ValidationError;
use std::fmt;

/// Where an Ollama server listens: an absolute http(s) URL, kept without a
/// trailing slash so paths can be appended to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OllamaUrl(String);

impl OllamaUrl {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self(value.into()))
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
