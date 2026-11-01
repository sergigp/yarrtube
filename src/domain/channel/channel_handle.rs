use crate::domain::shared::ValidationError;
use percent_encoding::percent_decode_str;
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

    /// YouTube treats handles that differ only in case (`@Name`, `@name`) as
    /// the same channel.
    pub fn is_same_channel_as(&self, other: &ChannelHandle) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }

    /// Parses a bare channel handle (`@name`) or a YouTube channel URL
    /// carrying a handle (`youtube.com/@name`) into a `ChannelHandle`.
    /// Anything else (an unrecognized host, a legacy `/channel/UC...` URL, a
    /// handle-less URL, an empty value) is rejected.
    ///
    /// A percent-encoded handle (`@%D0%9D...`, as a browser copies a
    /// non-Latin handle's URL) is decoded, so the same channel is always
    /// stored under the same handle however it was entered.
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
            return Self::new(decode(trimmed)?);
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
            Self::new(decode(segment)?)
        } else {
            Err(ValidationError(format!(
                "YouTube URL is missing a channel handle (got \"{value}\")"
            )))
        }
    }
}

fn decode(value: &str) -> Result<String, ValidationError> {
    percent_decode_str(value)
        .decode_utf8()
        .map(|decoded| decoded.into_owned())
        .map_err(|_| {
            ValidationError(format!(
                "Channel handle is not valid percent-encoded UTF-8 (got \"{value}\")"
            ))
        })
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

    #[test]
    fn it_should_keep_a_non_latin_handle_as_entered() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("@НаучныйСон"),
            Ok(ChannelHandle("@НаучныйСон".to_string()))
        );
    }

    #[test]
    fn it_should_decode_a_percent_encoded_handle() {
        assert_eq!(
            ChannelHandle::from_url_or_handle(
                "@%D0%9D%D0%B0%D1%83%D1%87%D0%BD%D1%8B%D0%B9%D0%A1%D0%BE%D0%BD"
            ),
            Ok(ChannelHandle("@НаучныйСон".to_string()))
        );
    }

    #[test]
    fn it_should_decode_a_percent_encoded_handle_in_a_youtube_url() {
        assert_eq!(
            ChannelHandle::from_url_or_handle(
                "https://www.youtube.com/@%D0%9D%D0%B0%D1%83%D1%87%D0%BD%D1%8B%D0%B9%D0%A1%D0%BE%D0%BD/videos"
            ),
            Ok(ChannelHandle("@НаучныйСон".to_string()))
        );
    }

    #[test]
    fn it_should_reject_a_handle_percent_encoding_invalid_utf8() {
        assert_eq!(
            ChannelHandle::from_url_or_handle("@%FF"),
            Err(error(
                "Channel handle is not valid percent-encoded UTF-8 (got \"@%FF\")"
            ))
        );
    }

    fn error(message: &str) -> ValidationError {
        ValidationError(message.to_string())
    }
}
