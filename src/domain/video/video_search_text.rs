use crate::domain::shared::ValidationError;

const MIN_LENGTH: usize = 2;

/// The text a video title search looks for, trimmed, at least 2 characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSearchText(String);

impl VideoSearchText {
    pub fn new(text: impl Into<String>) -> Result<Self, ValidationError> {
        let text = text.into().trim().to_string();
        if text.chars().count() < MIN_LENGTH {
            return Err(ValidationError(format!(
                "Search text must be at least {MIN_LENGTH} characters"
            )));
        }
        Ok(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_text_of_at_least_2_characters_and_trim_it() {
        assert_eq!(
            [
                VideoSearchText::new("ab"),
                VideoSearchText::new("  chess  ")
            ],
            [
                Ok(VideoSearchText("ab".to_string())),
                Ok(VideoSearchText("chess".to_string()))
            ]
        );
    }

    #[test]
    fn it_should_reject_a_text_shorter_than_2_characters_once_trimmed() {
        let invalid = ["", "a", " a ", "   "];

        assert_eq!(
            invalid.map(VideoSearchText::new),
            invalid.map(|_| Err(ValidationError(
                "Search text must be at least 2 characters".to_string()
            )))
        );
    }
}
