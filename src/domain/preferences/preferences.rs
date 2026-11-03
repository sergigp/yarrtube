use super::{PreferencesChange, SummarySettings, Theme};
use crate::domain::llm_server::OllamaUrl;
use crate::domain::shared::ValidationError;

/// The user's daemon-wide preferences.
#[derive(Debug, Clone, PartialEq)]
pub struct Preferences {
    pub categories_enabled: bool,
    pub home_show_continue_watching: bool,
    pub home_show_categories: bool,
    pub home_show_quick_watches: bool,
    pub home_show_latest: bool,
    pub theme: Theme,
    pub ai_enabled: bool,
    pub ollama_url: Option<OllamaUrl>,
    pub summaries: SummarySettings,
}

impl Default for Preferences {
    /// Every display preference on, in the light theme, with AI off.
    fn default() -> Self {
        Self {
            categories_enabled: true,
            home_show_continue_watching: true,
            home_show_categories: true,
            home_show_quick_watches: true,
            home_show_latest: true,
            theme: Theme::default(),
            ai_enabled: false,
            ollama_url: None,
            summaries: SummarySettings::default(),
        }
    }
}

impl Preferences {
    /// Category sections on home need categories on as well.
    pub fn shows_home_categories(&self) -> bool {
        self.categories_enabled && self.home_show_categories
    }

    /// These preferences with `change` applied, as long as the result is
    /// consistent: AI can't be on without an Ollama URL.
    pub fn apply(self, change: PreferencesChange) -> Result<Self, ValidationError> {
        let summaries = SummarySettings {
            enabled: change.summaries_enabled.unwrap_or(self.summaries.enabled),
            model: change.summaries_model.unwrap_or(self.summaries.model),
            context_length: change
                .summaries_context_length
                .unwrap_or(self.summaries.context_length),
            temperature: change
                .summaries_temperature
                .unwrap_or(self.summaries.temperature),
            thinking: change.summaries_thinking.unwrap_or(self.summaries.thinking),
            prompt: change.summaries_prompt.unwrap_or(self.summaries.prompt),
        };
        Self {
            categories_enabled: change.categories_enabled.unwrap_or(self.categories_enabled),
            home_show_continue_watching: change
                .home_show_continue_watching
                .unwrap_or(self.home_show_continue_watching),
            home_show_categories: change
                .home_show_categories
                .unwrap_or(self.home_show_categories),
            home_show_quick_watches: change
                .home_show_quick_watches
                .unwrap_or(self.home_show_quick_watches),
            home_show_latest: change.home_show_latest.unwrap_or(self.home_show_latest),
            theme: change.theme.unwrap_or(self.theme),
            ai_enabled: change.ai_enabled.unwrap_or(self.ai_enabled),
            ollama_url: change.ollama_url.unwrap_or(self.ollama_url),
            summaries,
        }
        .ensure_ai_has_url()
    }

    fn ensure_ai_has_url(self) -> Result<Self, ValidationError> {
        match (self.ai_enabled, &self.ollama_url) {
            (true, None) => Err(ValidationError(
                "Ollama URL must be set to enable AI".to_string(),
            )),
            _ => Ok(self),
        }
    }
}
