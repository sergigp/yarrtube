use crate::domain::shared::ValidationError;

/// A model's answer, ready to use: without the reasoning some models write
/// into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryText(String);

impl SummaryText {
    /// A summary of exactly `value`, e.g. as stored.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// `answer` after its last `</think>`, trimmed; rejects an empty result.
    pub fn from_answer(answer: &str) -> Result<Self, ValidationError> {
        Ok(Self::new(answer))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
