use crate::domain::shared::ValidationError;

/// Where a video stands in getting its original-language subtitles — see
/// the `video-subtitles` capability. `Unavailable` is terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtitleState {
    Missing,
    Fetched,
    Unavailable,
}

impl SubtitleState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Missing => "MISSING",
            Self::Fetched => "FETCHED",
            Self::Unavailable => "UNAVAILABLE",
        }
    }

    pub fn parse(value: &str) -> Result<Self, ValidationError> {
        match value {
            "MISSING" => Ok(Self::Missing),
            "FETCHED" => Ok(Self::Fetched),
            "UNAVAILABLE" => Ok(Self::Unavailable),
            other => Err(ValidationError(format!("unknown subtitle state '{other}'"))),
        }
    }
}
