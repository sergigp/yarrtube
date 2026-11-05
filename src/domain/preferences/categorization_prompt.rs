use crate::domain::shared::ValidationError;

const SHIPPED: &str = include_str!("../../../prompts/categorization/v1.md");

/// The prompt wrapping every category's own prompt: plain text, sent as is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorizationPrompt(String);

impl CategorizationPrompt {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self(value.into()))
    }

    /// A prompt as it was saved, without checking it: rules that change later
    /// must not lose the user's text.
    pub fn from_stored(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn shipped() -> Self {
        Self(SHIPPED.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
