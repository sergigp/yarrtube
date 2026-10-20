use crate::domain::shared::ValidationError;
use std::fmt;
use uuid::Uuid;

/// A category's identifier, generated when the category is created.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CategoryId(String);

impl CategoryId {
    /// Generates a new, unique identifier for a freshly created `Category`.
    pub fn new_generated() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Parses an identifier as stored or as supplied by a client.
    pub fn new(id: impl Into<String>) -> Result<Self, ValidationError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(ValidationError("Category ID must not be empty".to_string()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CategoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_an_id() {
        assert_eq!(
            CategoryId::new("category-1"),
            Ok(CategoryId("category-1".to_string()))
        );
    }

    #[test]
    fn it_should_reject_a_blank_id() {
        assert_eq!(
            CategoryId::new("  "),
            Err(ValidationError("Category ID must not be empty".to_string()))
        );
    }
}
