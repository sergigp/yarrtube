use crate::domain::shared::ValidationError;
use std::fmt;

/// A category's name: trimmed, 1 to `MAX_LENGTH` characters. Two names are
/// the same name when they are equal ignoring case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryName(String);

impl CategoryName {
    pub const MAX_LENGTH: usize = 60;

    pub fn new(name: impl Into<String>) -> Result<Self, ValidationError> {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            return Err(ValidationError(
                "Category name must not be empty".to_string(),
            ));
        }
        if name.chars().count() > Self::MAX_LENGTH {
            return Err(ValidationError(format!(
                "Category name must be at most {} characters",
                Self::MAX_LENGTH
            )));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Equal ignoring case.
    pub fn is_same_as(&self, other: &CategoryName) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

impl fmt::Display for CategoryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_name() {
        assert_eq!(
            CategoryName::new("Chess"),
            Ok(CategoryName("Chess".to_string()))
        );
    }

    #[test]
    fn it_should_trim_a_name() {
        assert_eq!(
            CategoryName::new("  Chess  "),
            Ok(CategoryName("Chess".to_string()))
        );
    }

    #[test]
    fn it_should_reject_an_empty_name() {
        assert_eq!(
            CategoryName::new(""),
            Err(error("Category name must not be empty"))
        );
    }

    #[test]
    fn it_should_reject_a_blank_name() {
        assert_eq!(
            CategoryName::new("   "),
            Err(error("Category name must not be empty"))
        );
    }

    #[test]
    fn it_should_accept_a_name_of_60_characters() {
        let name = "é".repeat(60);

        assert_eq!(CategoryName::new(name.as_str()), Ok(CategoryName(name)));
    }

    #[test]
    fn it_should_reject_a_name_of_61_characters() {
        assert_eq!(
            CategoryName::new("é".repeat(61)),
            Err(error("Category name must be at most 60 characters"))
        );
    }

    #[test]
    fn it_should_be_the_same_name_ignoring_case() {
        let chess = CategoryName::new("Chess").unwrap();

        assert_eq!(
            [
                chess.is_same_as(&CategoryName::new("cHESS").unwrap()),
                chess.is_same_as(&CategoryName::new("Chess club").unwrap()),
            ],
            [true, false]
        );
    }

    fn error(message: &str) -> ValidationError {
        ValidationError(message.to_string())
    }
}
