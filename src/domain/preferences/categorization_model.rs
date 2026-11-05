use crate::domain::shared::ValidationError;

/// The name of the model that categorizes videos, as the LLM server knows it
/// (e.g. `qwen3:4b`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorizationModel(String);

impl CategorizationModel {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self(value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for CategorizationModel {
    fn default() -> Self {
        Self("qwen3:4b".to_string())
    }
}
