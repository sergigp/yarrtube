use crate::domain::category::oklch::Oklch;
use crate::domain::shared::ValidationError;

/// Lightness above which a color reads as a pastel.
const LIGHT_LIGHTNESS: f64 = 0.85;
/// The palette's solid lightness and chroma.
const SOLID_LIGHTNESS: f64 = 0.62;
const SOLID_CHROMA: f64 = 0.13;

/// A category's color: `#` and 6 hexadecimal digits, lowercase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryColor(String);

impl CategoryColor {
    /// Solid presets, in the order defaults are handed out.
    pub const PALETTE: [&'static str; 10] = [
        "#f59e0b", "#f97316", "#ef4444", "#ec4899", "#8b5cf6", "#3b82f6", "#0ea5e9", "#14b8a6",
        "#22c55e", "#64748b",
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

    /// The solid color of this color's hue when this one is light (OKLCH
    /// lightness above 0.85), this color otherwise.
    pub fn strengthened(self) -> Self {
        let oklch = Oklch::from_hex(&self.0);
        if oklch.lightness <= LIGHT_LIGHTNESS {
            return self;
        }
        Self(
            Oklch {
                lightness: SOLID_LIGHTNESS,
                chroma: SOLID_CHROMA,
                ..oklch
            }
            .to_hex(),
        )
    }

    pub fn as_str(&self) -> &str {
        &self.0
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

    #[test]
    fn it_should_reject_short_named_and_non_hex_colors() {
        let invalid = [
            "#fff", "yellow", "#gggggg", "d3ecf8", "#d3ecf80", "", "#d3ecf ",
        ];

        assert_eq!(
            invalid.map(CategoryColor::new),
            invalid.map(|_| Err(ValidationError(
                "Category color must be # followed by 6 hexadecimal digits".to_string()
            )))
        );
    }
}
