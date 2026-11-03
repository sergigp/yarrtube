use crate::domain::shared::ValidationError;

/// How many tokens the model reads at once (Ollama's `num_ctx`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextLength(u32);

impl ContextLength {
    pub fn new(value: i64) -> Result<Self, ValidationError> {
        Ok(Self(value as u32))
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Default for ContextLength {
    fn default() -> Self {
        Self(12288)
    }
}
