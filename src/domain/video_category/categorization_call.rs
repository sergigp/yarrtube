use crate::domain::llm_server::{Effort, LlmModel};
use crate::domain::preferences::{
    AnthropicSettings, CategorizationPrompt, CategorizationSettings, ContextLength,
};

/// What each call of one categorization sends, resolved from the provider.
#[derive(Debug, Clone, PartialEq)]
pub struct CategorizationCall {
    pub model: String,
    pub prompt: CategorizationPrompt,
    pub context_length: ContextLength,
    /// Ollama only.
    pub temperature: f64,
    /// Anthropic only.
    pub effort: Effort,
}

impl CategorizationCall {
    /// The saved Ollama settings, at `context_length` (Auto resolved).
    pub fn ollama(settings: &CategorizationSettings, context_length: ContextLength) -> Self {
        Self {
            model: settings.model.as_str().to_string(),
            prompt: settings.effective_prompt(),
            context_length,
            temperature: settings.temperature.value(),
            effort: Effort::default(),
        }
    }

    /// The saved Anthropic settings with the shared `prompt`, for the model
    /// as Anthropic `listed` it.
    pub fn anthropic(
        settings: &AnthropicSettings,
        prompt: CategorizationPrompt,
        _listed: &LlmModel,
    ) -> Self {
        Self {
            model: settings.categorization_model.as_str().to_string(),
            prompt,
            context_length: ContextLength::default(),
            temperature: 0.0,
            effort: Effort::default(),
        }
    }
}
