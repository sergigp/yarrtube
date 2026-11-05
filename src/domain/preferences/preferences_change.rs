use super::{
    CategorizationModel, CategorizationPrompt, ContextLength, SummaryModel, SummaryPrompt,
    Temperature, Theme,
};
use crate::domain::llm_server::OllamaUrl;

/// Preferences to change; `None` keeps the stored value.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PreferencesChange {
    pub categories_enabled: Option<bool>,
    pub home_show_continue_watching: Option<bool>,
    pub home_show_categories: Option<bool>,
    pub home_show_quick_watches: Option<bool>,
    pub home_show_latest: Option<bool>,
    pub theme: Option<Theme>,
    pub ai_enabled: Option<bool>,
    /// `Some(None)` unsets the URL.
    pub ollama_url: Option<Option<OllamaUrl>>,
    pub summaries_enabled: Option<bool>,
    pub summaries_model: Option<SummaryModel>,
    pub summaries_context_length: Option<ContextLength>,
    pub summaries_temperature: Option<Temperature>,
    pub summaries_thinking: Option<bool>,
    /// `Some(None)` goes back to the shipped prompt.
    pub summaries_prompt: Option<Option<SummaryPrompt>>,
    pub categorization_enabled: Option<bool>,
    pub categorization_model: Option<CategorizationModel>,
    /// `Some(None)` goes back to the shipped prompt.
    pub categorization_prompt: Option<Option<CategorizationPrompt>>,
}
