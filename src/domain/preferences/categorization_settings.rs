use super::{CategorizationModel, CategorizationPrompt, ContextLength, Temperature};

/// How videos are categorized.
#[derive(Debug, Clone, PartialEq)]
pub struct CategorizationSettings {
    pub enabled: bool,
    pub model: CategorizationModel,
    pub context_length: ContextLength,
    pub temperature: Temperature,
    pub thinking: bool,
    /// The prompt the user saved; `None` uses the shipped one.
    pub prompt: Option<CategorizationPrompt>,
}

impl Default for CategorizationSettings {
    /// Off, with the settings `evals/categories` runs with.
    fn default() -> Self {
        Self {
            enabled: false,
            model: CategorizationModel::default(),
            context_length: ContextLength::categorization_default(),
            temperature: Temperature::categorization_default(),
            thinking: true,
            prompt: None,
        }
    }
}

impl CategorizationSettings {
    pub fn effective_prompt(&self) -> CategorizationPrompt {
        self.prompt
            .clone()
            .unwrap_or_else(CategorizationPrompt::shipped)
    }
}
