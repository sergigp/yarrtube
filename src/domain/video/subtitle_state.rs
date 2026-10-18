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
        "MISSING"
    }

    pub fn parse(_value: &str) -> Result<Self, ValidationError> {
        Ok(Self::Missing)
    }
}
