use super::errors::ChannelHandleError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelHandle(String);

impl ChannelHandle {
    pub fn new(handle: impl Into<String>) -> Result<Self, ChannelHandleError> {
        let handle = handle.into();
        let trimmed = handle.trim();
        if trimmed.is_empty() {
            return Err(ChannelHandleError(
                "Channel handle must not be empty".to_string(),
            ));
        }
        if !trimmed.starts_with('@') {
            return Err(ChannelHandleError(format!(
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
    pub fn from_url_or_handle(value: impl Into<String>) -> Result<Self, ChannelHandleError> {
        let value = value.into();
        let trimmed = value.trim();

        let Some(after_scheme) = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))
        else {
            if trimmed.is_empty() {
                return Err(ChannelHandleError(
                    "Channel handle or URL must not be empty".to_string(),
                ));
            }
            return Self::new(trimmed);
        };

        let (host, rest) = after_scheme.split_once('/').unwrap_or((after_scheme, ""));

        if host != "youtube.com" && host != "www.youtube.com" && host != "m.youtube.com" {
            return Err(ChannelHandleError(format!(
                "\"{value}\" is not a recognized YouTube channel URL"
            )));
        }

        let path = rest.split(['?', '#']).next().unwrap_or("");
        let segment = path.trim_matches('/').split('/').next().unwrap_or("");

        if segment.starts_with('@') && segment.len() > 1 {
            Self::new(segment)
        } else {
            Err(ChannelHandleError(format!(
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
        let handle = ChannelHandle::new("@somechannel").unwrap();
        assert_eq!(handle.as_str(), "@somechannel");
    }

    #[test]
    fn it_should_reject_an_empty_handle() {
        assert!(ChannelHandle::new("").is_err());
        assert!(ChannelHandle::new("   ").is_err());
    }

    #[test]
    fn it_should_reject_a_handle_missing_its_leading_at() {
        assert!(ChannelHandle::new("somechannel").is_err());
    }

    #[test]
    fn it_should_accept_a_bare_handle_via_from_url_or_handle() {
        let handle = ChannelHandle::from_url_or_handle("@somechannel").unwrap();
        assert_eq!(handle.as_str(), "@somechannel");
    }

    #[test]
    fn it_should_parse_a_full_youtube_channel_url() {
        let handle =
            ChannelHandle::from_url_or_handle("https://www.youtube.com/@somechannel").unwrap();
        assert_eq!(handle.as_str(), "@somechannel");
    }

    #[test]
    fn it_should_parse_a_youtube_channel_url_with_a_trailing_path_segment() {
        let handle =
            ChannelHandle::from_url_or_handle("https://youtube.com/@somechannel/videos").unwrap();
        assert_eq!(handle.as_str(), "@somechannel");
    }

    #[test]
    fn it_should_reject_a_handle_missing_its_leading_at_via_from_url_or_handle() {
        assert!(ChannelHandle::from_url_or_handle("somechannel").is_err());
    }

    #[test]
    fn it_should_reject_an_unrecognized_url() {
        assert!(ChannelHandle::from_url_or_handle("https://example.com/@somechannel").is_err());
    }

    #[test]
    fn it_should_reject_a_legacy_channel_id_url() {
        assert!(
            ChannelHandle::from_url_or_handle(
                "https://www.youtube.com/channel/UCabcdefghijklmnopqrstuv"
            )
            .is_err()
        );
    }

    #[test]
    fn it_should_reject_a_youtube_url_without_a_handle() {
        assert!(ChannelHandle::from_url_or_handle("https://www.youtube.com/").is_err());
    }

    #[test]
    fn it_should_reject_an_empty_value_via_from_url_or_handle() {
        assert!(ChannelHandle::from_url_or_handle("").is_err());
        assert!(ChannelHandle::from_url_or_handle("   ").is_err());
    }
}
