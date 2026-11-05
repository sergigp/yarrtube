use crate::domain::shared::ValidationError;

/// What the user wrote to describe the videos a category should hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryPrompt(String);

impl CategoryPrompt {
    /// `None` when blank.
    pub fn new(value: impl Into<String>) -> Result<Option<Self>, ValidationError> {
        Ok(Some(Self(value.into())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
