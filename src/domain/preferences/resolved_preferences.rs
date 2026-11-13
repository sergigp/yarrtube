use super::{ContextLength, Preferences};
use crate::domain::category::Category;

/// The preferences as read, with what the categorization context length is
/// now: Auto depends on the categories, not only on what was saved.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPreferences {
    pub preferences: Preferences,
    pub categorization_context_length: ContextLength,
    /// Whether the daemon has an Anthropic key.
    pub anthropic_available: bool,
}

impl ResolvedPreferences {
    /// `preferences` with the categorization context length Auto sizes from
    /// the categories with an AI prompt among `categories`, read alongside
    /// whether Anthropic is available.
    pub fn new(
        preferences: Preferences,
        categories: &[Category],
        anthropic_available: bool,
    ) -> Self {
        let prompted = categories
            .iter()
            .filter(|category| category.ai_prompt.is_some())
            .count();
        Self {
            categorization_context_length: preferences
                .categorization
                .effective_context_length(prompted),
            preferences,
            anthropic_available,
        }
    }
}
