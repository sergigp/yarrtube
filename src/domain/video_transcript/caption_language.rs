use crate::domain::shared::ValidationError;

/// A caption track's language code as YouTube reports it, e.g. `en` or
/// `pt-BR`; never carries yt-dlp's `-orig` marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionLanguage(String);

impl CaptionLanguage {
    pub fn new(value: &str) -> Result<Self, ValidationError> {
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
