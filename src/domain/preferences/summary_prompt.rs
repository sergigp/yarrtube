use crate::domain::shared::ValidationError;

/// The prompt shipped with this version; used until the user saves one.
const SHIPPED: &str = include_str!("../../../prompts/summaries/v3.md");

/// The summary prompt document: a free-form note, then one `## <heading>`
/// section per `SummaryPromptSection`, each using only its own variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryPrompt(String);

impl SummaryPrompt {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self(value.into()))
    }

    pub fn shipped() -> Self {
        Self(SHIPPED.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
