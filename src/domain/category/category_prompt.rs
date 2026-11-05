use crate::domain::shared::ValidationError;

const MAX_LENGTH: usize = 2000;

/// What the user wrote to describe the videos a category should hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryPrompt(String);

impl CategoryPrompt {
    /// Trimmed; `None` when blank.
    pub fn new(value: impl Into<String>) -> Result<Option<Self>, ValidationError> {
        let value = value.into();
        let trimmed = value.trim();
        match trimmed.chars().count() {
            0 => Ok(None),
            length if length > MAX_LENGTH => Err(ValidationError(format!(
                "Category AI prompt must be at most {MAX_LENGTH} characters (got {length})"
            ))),
            _ => Ok(Some(Self(trimmed.to_string()))),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_trim_the_prompt() {
        assert_eq!(
            CategoryPrompt::new("  Chess games\n").map(|prompt| prompt.map(|p| p.0)),
            Ok(Some("Chess games".to_string()))
        );
    }

    #[test]
    fn it_should_read_blank_text_as_no_prompt() {
        assert_eq!(CategoryPrompt::new(" \n "), Ok(None));
    }

    #[test]
    fn it_should_accept_a_prompt_of_2000_characters() {
        let text = "é".repeat(2000);

        assert_eq!(
            CategoryPrompt::new(text.clone()).map(|prompt| prompt.map(|p| p.0)),
            Ok(Some(text))
        );
    }

    #[test]
    fn it_should_reject_a_prompt_over_2000_characters() {
        assert_eq!(
            CategoryPrompt::new("é".repeat(2001)),
            Err(ValidationError(
                "Category AI prompt must be at most 2000 characters (got 2001)".to_string()
            ))
        );
    }
}
