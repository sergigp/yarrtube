use crate::domain::shared::ValidationError;

const SHIPPED: &str = include_str!("../../../prompts/categorization/v1.md");

/// The prompt wrapping every category's own prompt: plain text, sent as is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorizationPrompt(String);

impl CategorizationPrompt {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self(value.into()))
    }

    pub fn shipped() -> Self {
        Self(SHIPPED.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
