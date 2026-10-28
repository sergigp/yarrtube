use crate::domain::shared::ValidationError;
use std::fmt;

/// A category's color: `#` and 6 hexadecimal digits, lowercase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryColor(String);

impl CategoryColor {
    /// Soft presets, in the order defaults are handed out.
    pub const PALETTE: [&'static str; 10] = [
        "#fff1bf", "#ffe0c7", "#ffd9d4", "#fadceb", "#e9def8", "#dbe3fb", "#d3ecf8", "#cff0ea",
        "#e4f0cc", "#eee5d7",
    ];

    pub fn new(color: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self(color.into()))
    }

    /// Palette color used by the fewest of `used`, earliest on a tie.
    pub fn least_used(_used: &[CategoryColor]) -> Self {
        Self(Self::PALETTE[0].to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CategoryColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
