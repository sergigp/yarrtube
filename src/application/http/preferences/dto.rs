use crate::domain::preferences::Preferences;
use serde::{Deserialize, Deserializer, Serialize};

/// A partial update: every field left out keeps its stored value.
#[derive(Debug, Default, Deserialize)]
pub struct UpdatePreferencesRequest {
    #[serde(default)]
    pub categories_enabled: Option<bool>,
    #[serde(default)]
    pub home_show_continue_watching: Option<bool>,
    #[serde(default)]
    pub home_show_categories: Option<bool>,
    #[serde(default)]
    pub home_show_quick_watches: Option<bool>,
    #[serde(default)]
    pub home_show_latest: Option<bool>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub ai_enabled: Option<bool>,
    /// `""` unsets the URL.
    #[serde(default)]
    pub ollama_url: Option<String>,
    #[serde(default)]
    pub summaries_enabled: Option<bool>,
    #[serde(default)]
    pub summaries_model: Option<String>,
    #[serde(default)]
    pub summaries_context_length: Option<i64>,
    #[serde(default)]
    pub summaries_temperature: Option<f64>,
    #[serde(default)]
    pub summaries_thinking: Option<bool>,
    /// Left out keeps the prompt, `null` goes back to the shipped one.
    #[serde(default, deserialize_with = "present")]
    pub summaries_prompt: Option<Option<String>>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PreferencesResponse {
    pub categories_enabled: bool,
    pub home_show_continue_watching: bool,
    pub home_show_categories: bool,
    pub home_show_quick_watches: bool,
    pub home_show_latest: bool,
    pub theme: String,
    pub ai_enabled: bool,
    pub ollama_url: Option<String>,
    pub summaries_enabled: bool,
    pub summaries_model: String,
    pub summaries_context_length: u32,
    pub summaries_temperature: f64,
    pub summaries_thinking: bool,
    pub summaries_prompt: String,
    pub summaries_prompt_is_default: bool,
}

impl From<Preferences> for PreferencesResponse {
    fn from(preferences: Preferences) -> Self {
        let summaries = &preferences.summaries;
        Self {
            categories_enabled: preferences.categories_enabled,
            home_show_continue_watching: preferences.home_show_continue_watching,
            home_show_categories: preferences.home_show_categories,
            home_show_quick_watches: preferences.home_show_quick_watches,
            home_show_latest: preferences.home_show_latest,
            theme: preferences.theme.as_str().to_string(),
            ai_enabled: preferences.ai_enabled,
            ollama_url: preferences.ollama_url.as_ref().map(|url| url.to_string()),
            summaries_enabled: summaries.enabled,
            summaries_model: summaries.model.as_str().to_string(),
            summaries_context_length: summaries.context_length.value(),
            summaries_temperature: summaries.temperature.value(),
            summaries_thinking: summaries.thinking,
            summaries_prompt: summaries.effective_prompt().as_str().to_string(),
            summaries_prompt_is_default: summaries.prompt.is_none(),
        }
    }
}

/// Tells a field sent as `null` (`Some(None)`) from one left out (`None`,
/// via `#[serde(default)]`).
fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}
