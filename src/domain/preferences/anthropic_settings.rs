use super::{CategorizationModel, SummaryModel};
use crate::domain::llm_server::Effort;

/// How each AI feature calls Anthropic. The prompts are shared with Ollama.
#[derive(Debug, Clone, PartialEq)]
pub struct AnthropicSettings {
    pub summaries_model: SummaryModel,
    pub summaries_effort: Effort,
    pub categorization_model: CategorizationModel,
    pub categorization_effort: Effort,
}

impl Default for AnthropicSettings {
    fn default() -> Self {
        Self {
            summaries_model: SummaryModel::anthropic_default(),
            summaries_effort: Effort::default(),
            categorization_model: CategorizationModel::anthropic_default(),
            categorization_effort: Effort::default(),
        }
    }
}
