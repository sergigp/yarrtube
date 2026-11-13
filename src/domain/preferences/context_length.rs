use crate::domain::shared::ValidationError;

const MIN: u32 = 1024;
const MAX: u32 = 131072;
const DEFAULT: u32 = 12288;
const AUTO_MIN: u32 = 4096;
/// Room for one category's prompt, thinking and answer in a call about all
/// of them: 16,384 for the 9 that `evals/categories` measured.
const AUTO_TOKENS_PER_CATEGORY: u32 = 1820;
const AUTO_STEP: u32 = 1024;
const INPUT_BUDGET_OF_DEFAULT: u32 = 9000;

/// How many tokens the model reads at once (Ollama's `num_ctx`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextLength(u32);

impl ContextLength {
    pub fn new(value: i64) -> Result<Self, ValidationError> {
        match u32::try_from(value) {
            Ok(length) if (MIN..=MAX).contains(&length) => Ok(Self(length)),
            _ => Err(ValidationError(format!(
                "Context length must be between {MIN} and {MAX} (got {value})"
            ))),
        }
    }

    /// The categorization context length for `prompted_categories`
    /// categories with an AI prompt, when the user hasn't set one.
    pub fn auto(prompted_categories: usize) -> Self {
        let categories = u32::try_from(prompted_categories).unwrap_or(u32::MAX);
        let needed = AUTO_TOKENS_PER_CATEGORY.saturating_mul(categories);
        Self(
            needed
                .div_ceil(AUTO_STEP)
                .saturating_mul(AUTO_STEP)
                .clamp(AUTO_MIN, MAX),
        )
    }

    /// The most of a model's `window` (none: unknown) the input budget is
    /// computed from: at most the longest length, 131072.
    pub fn of_window(window: Option<u32>) -> Self {
        Self(window.map_or(MAX, |window| window.clamp(MIN, MAX)))
    }

    pub fn value(&self) -> u32 {
        self.0
    }

    /// The tokens the prompt can take, leaving room for thinking and the
    /// answer: 9000 of the default 12288, as measured in `evals/summaries`.
    pub fn input_budget_tokens(&self) -> u32 {
        self.0 * INPUT_BUDGET_OF_DEFAULT / DEFAULT
    }
}

impl Default for ContextLength {
    fn default() -> Self {
        Self(DEFAULT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_the_minimum() {
        assert_eq!(ContextLength::new(1024).map(|c| c.value()), Ok(1024));
    }

    #[test]
    fn it_should_accept_the_maximum() {
        assert_eq!(ContextLength::new(131072).map(|c| c.value()), Ok(131072));
    }

    #[test]
    fn it_should_reject_below_the_minimum() {
        assert_eq!(ContextLength::new(1023), Err(rejected(1023)));
    }

    #[test]
    fn it_should_reject_above_the_maximum() {
        assert_eq!(ContextLength::new(131073), Err(rejected(131073)));
    }

    #[test]
    fn it_should_reject_a_negative_length() {
        assert_eq!(ContextLength::new(-1), Err(rejected(-1)));
    }

    #[test]
    fn it_should_size_auto_from_the_categories_with_a_prompt() {
        assert_eq!(
            [0, 2, 3, 9, 10, 80].map(|categories| ContextLength::auto(categories).value()),
            [4096, 4096, 6144, 16384, 18432, 131072]
        );
    }

    #[test]
    fn it_should_cap_a_model_window_at_the_maximum() {
        assert_eq!(
            [Some(1_000_000), Some(200_000), Some(32_768), Some(10), None]
                .map(|window| ContextLength::of_window(window).value()),
            [131072, 131072, 32768, 1024, 131072]
        );
    }

    fn rejected(value: i64) -> ValidationError {
        ValidationError(format!(
            "Context length must be between 1024 and 131072 (got {value})"
        ))
    }
}
