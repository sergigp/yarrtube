use crate::domain::shared::ValidationError;

/// The English names of the languages summaries are likeliest to be written
/// in, by base language code.
const LANGUAGE_NAMES: [(&str, &str); 24] = [
    ("ar", "Arabic"),
    ("ca", "Catalan"),
    ("cs", "Czech"),
    ("da", "Danish"),
    ("de", "German"),
    ("el", "Greek"),
    ("en", "English"),
    ("es", "Spanish"),
    ("fi", "Finnish"),
    ("fr", "French"),
    ("hi", "Hindi"),
    ("id", "Indonesian"),
    ("it", "Italian"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
    ("nl", "Dutch"),
    ("no", "Norwegian"),
    ("pl", "Polish"),
    ("pt", "Portuguese"),
    ("ro", "Romanian"),
    ("ru", "Russian"),
    ("sv", "Swedish"),
    ("tr", "Turkish"),
    ("zh", "Chinese"),
];

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

    /// `Spanish` for `es-US`; the code itself when unknown.
    pub fn english_name(&self) -> String {
        LANGUAGE_NAMES
            .iter()
            .find(|(code, _)| *code == self.base())
            .map_or_else(|| self.0.clone(), |(_, name)| name.to_string())
    }

    /// 4.3 for `en` and 3.4 by default (measured on the eval golden set),
    /// 1.5 for `ja`, `zh` and `ko` (measured on `qwen3:4b`, about 1.9 on
    /// written text, lowered as captions run denser).
    pub fn chars_per_token(&self) -> f64 {
        match self.base() {
            "en" => 4.3,
            "ja" | "zh" | "ko" => 1.5,
            _ => 3.4,
        }
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
    fn it_should_estimate_japanese_chinese_and_korean_at_fewer_characters_per_token() {
        let estimates = ["ja", "zh-Hans", "ko", "en-US", "es"]
            .map(|code| CaptionLanguage::new(code).unwrap().chars_per_token());

        assert_eq!(estimates, [1.5, 1.5, 1.5, 4.3, 3.4]);
    }

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
