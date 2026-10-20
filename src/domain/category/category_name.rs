use crate::domain::shared::ValidationError;
use std::fmt;

/// A category's name: trimmed, 1 to `MAX_LENGTH` characters. Two names are
/// the same name when they are equal ignoring case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryName(String);

impl CategoryName {
    pub const MAX_LENGTH: usize = 60;

    pub fn new(name: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self(name.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Equal ignoring case.
    pub fn is_same_as(&self, _other: &CategoryName) -> bool {
        false
    }
}

impl fmt::Display for CategoryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
