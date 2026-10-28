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
        let color = color.into().to_lowercase();
        let digits = color.strip_prefix('#').unwrap_or_default();
        if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ValidationError(
                "Category color must be # followed by 6 hexadecimal digits".to_string(),
            ));
        }
        Ok(Self(color))
    }

    /// Palette color used by the fewest of `used`, earliest on a tie.
    pub fn least_used(used: &[CategoryColor]) -> Self {
        let uses = |hex: &str| used.iter().filter(|color| color.as_str() == hex).count();
        let least = Self::PALETTE
            .into_iter()
            .min_by_key(|hex| uses(hex))
            .unwrap_or(Self::PALETTE[0]);
        Self(least.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_hex_color_and_lowercase_it() {
        assert_eq!(
            [CategoryColor::new("#d3ecf8"), CategoryColor::new("#D3eCF8")],
            [
                Ok(CategoryColor("#d3ecf8".to_string())),
                Ok(CategoryColor("#d3ecf8".to_string()))
            ]
        );
    }
}
