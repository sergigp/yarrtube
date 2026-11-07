use super::{CategorizationModel, CategorizationPrompt};

/// How videos are categorized.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CategorizationSettings {
    pub enabled: bool,
    pub model: CategorizationModel,
    /// The prompt the user saved; `None` uses the shipped one.
    pub prompt: Option<CategorizationPrompt>,
}

impl CategorizationSettings {
    pub fn effective_prompt(&self) -> CategorizationPrompt {
        self.prompt
            .clone()
            .unwrap_or_else(CategorizationPrompt::shipped)
    }
}
