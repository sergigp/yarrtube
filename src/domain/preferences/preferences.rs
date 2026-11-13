use super::{
    AiProvider, AnthropicSettings, CategorizationSettings, PreferencesChange, SummarySettings,
    Theme,
};
use crate::domain::llm_server::{LlmConnection, OllamaUrl};
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
    pub ai_provider: AiProvider,
    pub ollama_url: Option<OllamaUrl>,
    pub summaries: SummarySettings,
    pub categorization: CategorizationSettings,
    pub anthropic: AnthropicSettings,
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
            ai_provider: AiProvider::default(),
            ollama_url: None,
            summaries: SummarySettings::default(),
            categorization: CategorizationSettings::default(),
            anthropic: AnthropicSettings::default(),
        }
    }
}

impl Preferences {
    /// Category sections on home need categories on as well.
    pub fn shows_home_categories(&self) -> bool {
        self.categories_enabled && self.home_show_categories
    }

    /// Whether sources can have their videos summarized: AI and summaries on.
    pub fn summaries_available(&self) -> bool {
        self.ai_enabled && self.summaries.enabled
    }

    /// Whether sources can have their videos categorized: AI, categories and
    /// categorization on.
    pub fn categorization_available(&self) -> bool {
        self.ai_enabled && self.categories_enabled && self.categorization.enabled
    }

    /// Where AI calls go: `None` only for Ollama without a URL.
    pub fn connection(&self) -> Option<LlmConnection> {
        self.ollama_url.clone().map(LlmConnection::Ollama)
    }

    /// Whether `other` generates summaries differently: another provider,
    /// server or other settings, not merely summaries turned on or off.
    pub fn changes_summaries(&self, other: &Self) -> bool {
        let settings = SummarySettings {
            enabled: other.summaries.enabled,
            ..self.summaries.clone()
        };
        self.ai_provider != other.ai_provider
            || self.ollama_url != other.ollama_url
            || settings != other.summaries
            || self.anthropic.summaries_model != other.anthropic.summaries_model
            || self.anthropic.summaries_effort != other.anthropic.summaries_effort
    }

    /// Whether `other` categorizes differently: another provider, server or
    /// other settings (model, context length as saved, temperature, thinking,
    /// effective prompt, Anthropic model and effort), not merely
    /// categorization turned on or off.
    pub fn changes_categorization(&self, other: &Self) -> bool {
        let calls = |settings: &CategorizationSettings| CategorizationSettings {
            enabled: false,
            prompt: Some(settings.effective_prompt()),
            ..settings.clone()
        };
        self.ai_provider != other.ai_provider
            || self.ollama_url != other.ollama_url
            || calls(&self.categorization) != calls(&other.categorization)
            || self.anthropic.categorization_model != other.anthropic.categorization_model
            || self.anthropic.categorization_effort != other.anthropic.categorization_effort
    }

    /// These preferences with `change` applied, as long as the result is
    /// consistent: AI can't be on Ollama without a URL, and Anthropic can't
    /// be chosen unless `anthropic_available`.
    pub fn apply(
        self,
        change: PreferencesChange,
        anthropic_available: bool,
    ) -> Result<Self, ValidationError> {
        self.ensure_anthropic_has_key(&change, anthropic_available)?;
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
        let categorization = CategorizationSettings {
            enabled: change
                .categorization_enabled
                .unwrap_or(self.categorization.enabled),
            model: change
                .categorization_model
                .unwrap_or(self.categorization.model),
            context_length: change
                .categorization_context_length
                .unwrap_or(self.categorization.context_length),
            temperature: change
                .categorization_temperature
                .unwrap_or(self.categorization.temperature),
            thinking: change
                .categorization_thinking
                .unwrap_or(self.categorization.thinking),
            prompt: change
                .categorization_prompt
                .unwrap_or(self.categorization.prompt),
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
            ai_provider: change.ai_provider.unwrap_or(self.ai_provider),
            ollama_url: change.ollama_url.unwrap_or(self.ollama_url),
            summaries,
            categorization,
            anthropic: AnthropicSettings {
                summaries_model: change
                    .anthropic_summaries_model
                    .unwrap_or(self.anthropic.summaries_model),
                summaries_effort: change
                    .anthropic_summaries_effort
                    .unwrap_or(self.anthropic.summaries_effort),
                categorization_model: change
                    .anthropic_categorization_model
                    .unwrap_or(self.anthropic.categorization_model),
                categorization_effort: change
                    .anthropic_categorization_effort
                    .unwrap_or(self.anthropic.categorization_effort),
            },
        }
        .ensure_ai_has_url()
    }

    /// Without a key, Anthropic can't be picked, nor AI turned on with it.
    /// Other changes are kept while it stays saved, so a removed key doesn't
    /// block every save.
    fn ensure_anthropic_has_key(
        &self,
        change: &PreferencesChange,
        anthropic_available: bool,
    ) -> Result<(), ValidationError> {
        let provider = change.ai_provider.unwrap_or(self.ai_provider);
        let starts_using = change.ai_provider.is_some() || change.ai_enabled == Some(true);
        match (provider, starts_using, anthropic_available) {
            (AiProvider::Anthropic, true, false) => Err(ValidationError(
                "Set YARRTUBE_ANTHROPIC_API_KEY to use Anthropic".to_string(),
            )),
            _ => Ok(()),
        }
    }

    fn ensure_ai_has_url(self) -> Result<Self, ValidationError> {
        match (self.ai_enabled, self.ai_provider, &self.ollama_url) {
            (true, AiProvider::Ollama, None) => Err(ValidationError(
                "Ollama URL must be set to enable AI".to_string(),
            )),
            _ => Ok(self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_have_summaries_available_only_with_ai_and_summaries_on() {
        let availability = [(false, false), (false, true), (true, false), (true, true)].map(
            |(ai_enabled, summaries_enabled)| {
                preferences(ai_enabled, summaries_enabled).summaries_available()
            },
        );

        assert_eq!(availability, [false, false, false, true]);
    }

    fn preferences(ai_enabled: bool, summaries_enabled: bool) -> Preferences {
        let preferences = Preferences::default();
        Preferences {
            ai_enabled,
            summaries: SummarySettings {
                enabled: summaries_enabled,
                ..preferences.summaries.clone()
            },
            ..preferences
        }
    }
}
