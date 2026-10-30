use crate::domain::shared::ValidationError;
use std::fmt;

/// The color theme the web UI is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl Theme {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        match value.as_str() {
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            _ => Err(ValidationError(format!(
                "Theme must be \"light\" or \"dark\" (got \"{value}\")"
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_parse_light() {
        assert_eq!(Theme::new("light"), Ok(Theme::Light));
    }

    #[test]
    fn it_should_parse_dark() {
        assert_eq!(Theme::new("dark"), Ok(Theme::Dark));
    }

    #[test]
    fn it_should_reject_an_unknown_theme() {
        assert_eq!(
            Theme::new("Dark"),
            Err(ValidationError(
                "Theme must be \"light\" or \"dark\" (got \"Dark\")".to_string()
            ))
        );
    }
}
