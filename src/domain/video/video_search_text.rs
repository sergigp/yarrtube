use crate::domain::shared::ValidationError;

/// The text a video title search looks for, trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSearchText(String);

impl VideoSearchText {
    pub fn new(text: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self(text.into().trim().to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
