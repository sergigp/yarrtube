use crate::domain::shared::ValidationError;

const SHIPPED: &str = include_str!("../../../prompts/categorization/v1.md");

/// The prompt wrapping every category's own prompt: plain text, sent as is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorizationPrompt(String);

impl CategorizationPrompt {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        match value.trim().is_empty() {
            true => Err(ValidationError(
                "Categorization prompt must not be empty".to_string(),
            )),
            false => Ok(Self(value)),
        }
    }

    /// A prompt as it was saved, without checking it: rules that change later
    /// must not lose the user's text.
    pub fn from_stored(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn shipped() -> Self {
        Self(SHIPPED.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_any_text() {
        assert_eq!(
            CategorizationPrompt::new(" {Answer} yes\n").map(|prompt| prompt.as_str().to_string()),
            Ok(" {Answer} yes\n".to_string())
        );
    }

    #[test]
    fn it_should_reject_a_blank_prompt() {
        assert_eq!(
            CategorizationPrompt::new(" \n "),
            Err(ValidationError(
                "Categorization prompt must not be empty".to_string()
            ))
        );
    }

    #[test]
    fn it_should_ship_a_valid_prompt() {
        assert_eq!(
            CategorizationPrompt::new(SHIPPED),
            Ok(CategorizationPrompt::shipped())
        );
    }
}
