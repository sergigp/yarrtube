use crate::domain::shared::ValidationError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelHandle(String);

impl ChannelHandle {
    pub fn new(handle: impl Into<String>) -> Result<Self, ValidationError> {
        let handle = handle.into();
        let trimmed = handle.trim();
        if trimmed.is_empty() {
            return Err(ValidationError(
                "Channel handle must not be empty".to_string(),
            ));
        }
        if !trimmed.starts_with('@') {
            return Err(ValidationError(format!(
                "Channel handle must start with \"@\" (got \"{trimmed}\")"
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parses a bare channel handle (`@name`) or a YouTube channel URL
    /// carrying a handle (`youtube.com/@name`) into a `ChannelHandle`.
    /// Anything else (an unrecognized host, a legacy `/channel/UC...` URL, a
    /// handle-less URL, an empty value) is rejected.
    pub fn from_url_or_handle(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        let trimmed = value.trim();

        let Some(after_scheme) = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))
        else {
            if trimmed.is_empty() {
                return Err(ValidationError(
                    "Channel handle or URL must not be empty".to_string(),
                ));
            }
            return Self::new(trimmed);
        };

        let (host, rest) = after_scheme.split_once('/').unwrap_or((after_scheme, ""));

        if host != "youtube.com" && host != "www.youtube.com" && host != "m.youtube.com" {
            return Err(ValidationError(format!(
                "\"{value}\" is not a recognized YouTube channel URL"
            )));
        }

        let path = rest.split(['?', '#']).next().unwrap_or("");
        let segment = path.trim_matches('/').split('/').next().unwrap_or("");

        if segment.starts_with('@') && segment.len() > 1 {
            Self::new(segment)
        } else {
            Err(ValidationError(format!(
                "YouTube URL is missing a channel handle (got \"{value}\")"
            )))
        }
    }
}

impl fmt::Display for ChannelHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_bare_handle() {
        assert_eq!(
            ChannelHandle::new("@somechannel"),
            Ok(ChannelHandle("@somechannel".to_string()))
        );
    }

    #[test]
    fn it_should_trim_surrounding_whitespace_from_a_handle() {
        assert_eq!(
            ChannelHandle::new("  @somechannel  "),
            Ok(ChannelHandle("@somechannel".to_string()))
        );
    }

    #[test]
    fn it_should_reject_an_empty_handle() {
        assert_eq!(
            ChannelHandle::new(""),
            Err(error("Channel handle must not be empty"))
        );
    }

    #[test]
    fn it_should_reject_a_blank_handle() {
        assert_eq!(
            ChannelHandle::new("   "),
            Err(error("Channel handle must not be empty"))
        );
    }

    #[test]
    fn it_should_reject_a_handle_missing_its_leading_at() {
        assert_eq!(
            ChannelHandle::new("somechannel"),
            Err(error(
                "Channel handle must start with \"@\" (got \"somechannel\")"
            ))
        );
    }

    #[test]
    fn it_should_accept_a_bare_handle_via_from_url_or_handle() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("@somechannel"),
            Ok(ChannelHandle("@somechannel".to_string()))
        );
    }

    #[test]
    fn it_should_parse_a_www_youtube_channel_url() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("https://www.youtube.com/@somechannel"),
            Ok(ChannelHandle("@somechannel".to_string()))
        );
    }

    #[test]
    fn it_should_parse_a_bare_youtube_channel_url() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("https://youtube.com/@somechannel"),
            Ok(ChannelHandle("@somechannel".to_string()))
        );
    }

    #[test]
    fn it_should_parse_a_mobile_youtube_channel_url() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("https://m.youtube.com/@somechannel"),
            Ok(ChannelHandle("@somechannel".to_string()))
        );
    }

    #[test]
    fn it_should_parse_an_http_youtube_channel_url() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("http://www.youtube.com/@somechannel"),
            Ok(ChannelHandle("@somechannel".to_string()))
        );
    }

    #[test]
    fn it_should_parse_a_youtube_channel_url_with_a_trailing_path_segment() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("https://youtube.com/@somechannel/videos"),
            Ok(ChannelHandle("@somechannel".to_string()))
        );
    }

    #[test]
    fn it_should_parse_a_youtube_channel_url_with_a_query_string() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("https://www.youtube.com/@somechannel?si=abc"),
            Ok(ChannelHandle("@somechannel".to_string()))
        );
    }

    #[test]
    fn it_should_reject_an_empty_value_via_from_url_or_handle() {
        assert_eq!(
            ChannelHandle::from_url_or_handle(""),
            Err(error("Channel handle or URL must not be empty"))
        );
    }

    #[test]
    fn it_should_reject_a_blank_value_via_from_url_or_handle() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("   "),
            Err(error("Channel handle or URL must not be empty"))
        );
    }

    #[test]
    fn it_should_reject_a_handle_missing_its_leading_at_via_from_url_or_handle() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("somechannel"),
            Err(error(
                "Channel handle must start with \"@\" (got \"somechannel\")"
            ))
        );
    }

    #[test]
    fn it_should_reject_an_unrecognized_url() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("https://example.com/@somechannel"),
            Err(error(
                "\"https://example.com/@somechannel\" is not a recognized YouTube channel URL"
            ))
        );
    }

    #[test]
    fn it_should_reject_a_legacy_channel_id_url() {
        assert_eq!(
            ChannelHandle::from_url_or_handle(
                "https://www.youtube.com/channel/UCabcdefghijklmnopqrstuv"
            ),
            Err(error(
                "YouTube URL is missing a channel handle (got \"https://www.youtube.com/channel/UCabcdefghijklmnopqrstuv\")"
            ))
        );
    }

    #[test]
    fn it_should_reject_a_youtube_url_without_a_handle() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("https://www.youtube.com/"),
            Err(error(
                "YouTube URL is missing a channel handle (got \"https://www.youtube.com/\")"
            ))
        );
    }

    #[test]
    fn it_should_reject_a_youtube_url_with_a_bare_at_sign() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("https://www.youtube.com/@"),
            Err(error(
                "YouTube URL is missing a channel handle (got \"https://www.youtube.com/@\")"
            ))
        );
    }

    fn error(message: &str) -> ValidationError {
        ValidationError(message.to_string())
    }
}
