use super::{CategorizationModel, CategorizationPrompt, ContextLength, Temperature};

/// How videos are categorized.
#[derive(Debug, Clone, PartialEq)]
pub struct CategorizationSettings {
    pub enabled: bool,
    pub model: CategorizationModel,
    /// The length the user saved; `None` is Auto.
    pub context_length: Option<ContextLength>,
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
            context_length: None,
            temperature: Temperature::categorization_default(),
            thinking: true,
            prompt: None,
        }
    }
}

impl CategorizationSettings {
    /// The saved length, or Auto's for `prompted_categories` categories
    /// with an AI prompt.
    pub fn effective_context_length(&self, prompted_categories: usize) -> ContextLength {
        self.context_length
            .unwrap_or_else(|| ContextLength::auto(prompted_categories))
    }

    pub fn effective_prompt(&self) -> CategorizationPrompt {
        self.prompt
            .clone()
            .unwrap_or_else(CategorizationPrompt::shipped)
    }
}
