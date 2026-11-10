use super::{ContextLength, Preferences};

/// The preferences as read, with what the categorization context length is
/// now: Auto depends on the categories, not only on what was saved.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPreferences {
    pub preferences: Preferences,
    pub categorization_context_length: ContextLength,
}
