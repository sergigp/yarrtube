use crate::domain::shared::ValidationError;

/// A caption track's language code as YouTube reports it, e.g. `en` or
/// `pt-BR`; never carries yt-dlp's `-orig` marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionLanguage(String);

impl CaptionLanguage {
    pub fn new(value: &str) -> Result<Self, ValidationError> {
        if value.is_empty() {
            return Err(ValidationError(
                "Caption language must not be empty".to_string(),
            ));
        }
        if value.ends_with("-orig") {
            return Err(ValidationError(format!(
                "Caption language must not carry the \"-orig\" marker (got \"{value}\")"
            )));
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The primary subtag, e.g. `es` for `es-US`.
    pub fn base(&self) -> &str {
        self.0.split('-').next().unwrap_or(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_language_code() {
        assert_eq!(
            ["en", "pt-BR"].map(|code| CaptionLanguage::new(code).map(|l| l.as_str().to_string())),
            [Ok("en".to_string()), Ok("pt-BR".to_string())]
        );
    }

    #[test]
    fn it_should_reject_an_empty_language() {
        assert_eq!(
            CaptionLanguage::new(""),
            Err(ValidationError(
                "Caption language must not be empty".to_string()
            ))
        );
    }

    #[test]
    fn it_should_reject_an_original_track_marker() {
        assert_eq!(
            CaptionLanguage::new("en-orig"),
            Err(ValidationError(
                "Caption language must not carry the \"-orig\" marker (got \"en-orig\")"
                    .to_string()
            ))
        );
    }

    #[test]
    fn it_should_return_the_base_language() {
        assert_eq!(
            ["es-US", "en"].map(|code| CaptionLanguage::new(code).unwrap().base().to_string()),
            ["es".to_string(), "en".to_string()]
        );
    }
}
