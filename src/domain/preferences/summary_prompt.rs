use super::SummaryPromptSection;
use crate::domain::shared::ValidationError;

/// The prompt shipped with this version; used until the user saves one.
const SHIPPED: &str = include_str!("../../../prompts/summaries/v3.md");

/// The summary prompt document: a free-form note, then one `## <heading>`
/// section per `SummaryPromptSection`, each using only its own variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryPrompt(String);

impl SummaryPrompt {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        let headings = Self::headings(&value);
        let missing = SummaryPromptSection::ALL
            .into_iter()
            .find(|section| !headings.contains(&section.heading()));
        match missing {
            Some(section) => Err(ValidationError(format!(
                "Prompt is missing the \"{}\" section",
                section.heading()
            ))),
            None => Ok(Self(value)),
        }
    }

    pub fn shipped() -> Self {
        Self(SHIPPED.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The heading of every `## <heading>` line, in order.
    fn headings(value: &str) -> Vec<&str> {
        value
            .lines()
            .filter_map(|line| line.strip_prefix("## "))
            .map(str::trim)
            .collect()
    }
}
