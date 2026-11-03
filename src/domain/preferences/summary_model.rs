use crate::domain::shared::ValidationError;

/// The name of the model that writes summaries, as the LLM server knows it
/// (e.g. `qwen3:4b`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryModel(String);

impl SummaryModel {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self(value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SummaryModel {
    fn default() -> Self {
        Self("qwen3:4b".to_string())
    }
}
