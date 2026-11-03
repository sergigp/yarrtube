use crate::domain::shared::ValidationError;
use std::fmt;

/// Where an Ollama server listens: an absolute http(s) URL, kept without a
/// trailing slash so paths can be appended to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OllamaUrl(String);

impl OllamaUrl {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        let trimmed = value.trim_end_matches('/');
        match Self::host_and_path(trimmed) {
            Some(rest) if !rest.is_empty() && !rest.contains(char::is_whitespace) => {
                Ok(Self(trimmed.to_string()))
            }
            _ => Err(ValidationError(format!(
                "Ollama URL must be an http(s) URL (got \"{value}\")"
            ))),
        }
    }

    /// What follows an `http://` or `https://` scheme; `None` for any other
    /// scheme.
    fn host_and_path(value: &str) -> Option<&str> {
        ["http://", "https://"]
            .iter()
            .find_map(|scheme| value.strip_prefix(scheme))
    }
}

impl fmt::Display for OllamaUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_an_http_url() {
        assert_eq!(
            OllamaUrl::new("http://mac.local:11434").map(|url| url.to_string()),
            Ok("http://mac.local:11434".to_string())
        );
    }

    #[test]
    fn it_should_accept_an_https_url() {
        assert_eq!(
            OllamaUrl::new("https://ollama.example.com").map(|url| url.to_string()),
            Ok("https://ollama.example.com".to_string())
        );
    }

    #[test]
    fn it_should_drop_trailing_slashes() {
        assert_eq!(
            OllamaUrl::new("http://mac.local:11434//").map(|url| url.to_string()),
            Ok("http://mac.local:11434".to_string())
        );
    }

    #[test]
    fn it_should_reject_a_url_without_scheme() {
        assert_eq!(
            OllamaUrl::new("mac.local:11434"),
            Err(rejected("mac.local:11434"))
        );
    }

    #[test]
    fn it_should_reject_another_scheme() {
        assert_eq!(
            OllamaUrl::new("ftp://mac.local"),
            Err(rejected("ftp://mac.local"))
        );
    }

    #[test]
    fn it_should_reject_an_empty_url() {
        assert_eq!(OllamaUrl::new(""), Err(rejected("")));
    }

    #[test]
    fn it_should_reject_a_url_without_host() {
        assert_eq!(OllamaUrl::new("http://"), Err(rejected("http://")));
    }

    #[test]
    fn it_should_reject_a_url_with_spaces() {
        assert_eq!(
            OllamaUrl::new("http://mac local"),
            Err(rejected("http://mac local"))
        );
    }

    fn rejected(value: &str) -> ValidationError {
        ValidationError(format!(
            "Ollama URL must be an http(s) URL (got \"{value}\")"
        ))
    }
}
