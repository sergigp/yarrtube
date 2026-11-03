use super::{ContextLength, SummaryModel, SummaryPrompt, Temperature};

/// How video summaries are generated.
#[derive(Debug, Clone, PartialEq)]
pub struct SummarySettings {
    pub enabled: bool,
    pub model: SummaryModel,
    pub context_length: ContextLength,
    pub temperature: Temperature,
    pub thinking: bool,
    /// The prompt the user saved; `None` uses the shipped one.
    pub prompt: Option<SummaryPrompt>,
}

impl Default for SummarySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            model: SummaryModel::default(),
            context_length: ContextLength::default(),
            temperature: Temperature::default(),
            thinking: true,
            prompt: None,
        }
    }
}

impl SummarySettings {
    pub fn effective_prompt(&self) -> SummaryPrompt {
        self.prompt.clone().unwrap_or_else(SummaryPrompt::shipped)
    }
}
