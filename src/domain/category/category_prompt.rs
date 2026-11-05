use crate::domain::shared::ValidationError;

/// What the user wrote to describe the videos a category should hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryPrompt(String);

impl CategoryPrompt {
    /// `None` when blank.
    pub fn new(value: impl Into<String>) -> Result<Option<Self>, ValidationError> {
        let value = value.into();
        let trimmed = value.trim();
        Ok((!trimmed.is_empty()).then(|| Self(trimmed.to_string())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
