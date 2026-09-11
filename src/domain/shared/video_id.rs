use super::errors::VideoIdError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VideoId(String);

impl VideoId {
    pub fn new(id: impl Into<String>) -> Result<Self, VideoIdError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(VideoIdError(
                "YouTube video ID must not be empty".to_string(),
            ));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_url(&self) -> String {
        format!("https://www.youtube.com/watch?v={}", self.0)
    }

    /// Parses a bare YouTube video ID or a full/short YouTube watch URL
    /// (`youtube.com/watch?v=...`, `youtu.be/...`) into a `VideoId`. Anything
    /// else (an unrecognized URL, an empty value) is rejected.
    pub fn from_url_or_id(value: impl Into<String>) -> Result<Self, VideoIdError> {
        let value = value.into();
        let trimmed = value.trim();

        let Some(after_scheme) = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))
        else {
            return Self::new(trimmed);
        };

        let (host, rest) = after_scheme.split_once('/').unwrap_or((after_scheme, ""));

        if host == "youtu.be" {
            let id = rest.split(['?', '&']).next().unwrap_or_default();
            return Self::new(id);
        }

        if host == "youtube.com" || host == "www.youtube.com" || host == "m.youtube.com" {
            let query = rest.split_once('?').map(|(_, query)| query).unwrap_or("");
            let id = query.split('&').find_map(|pair| pair.strip_prefix("v="));
            return match id {
                Some(id) if !id.is_empty() => Self::new(id),
                _ => Err(VideoIdError(format!(
                    "YouTube video URL is missing a \"v\" query parameter (got \"{value}\")"
                ))),
            };
        }

        Err(VideoIdError(format!(
            "\"{value}\" is not a recognized YouTube video URL"
        )))
    }
}

impl fmt::Display for VideoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_non_empty_id() {
        let id = VideoId::new("vid1").unwrap();
        assert_eq!(id.as_str(), "vid1");
    }

    #[test]
    fn it_should_reject_an_empty_id() {
        assert!(VideoId::new("").is_err());
        assert!(VideoId::new("   ").is_err());
    }

    #[test]
    fn it_should_build_the_youtube_video_url() {
        let id = VideoId::new("vid1").unwrap();
        assert_eq!(id.to_url(), "https://www.youtube.com/watch?v=vid1");
    }

    #[test]
    fn it_should_accept_a_bare_id_via_from_url_or_id() {
        let id = VideoId::from_url_or_id("vid1").unwrap();
        assert_eq!(id.as_str(), "vid1");
    }

    #[test]
    fn it_should_parse_a_full_youtube_watch_url() {
        let id = VideoId::from_url_or_id("https://www.youtube.com/watch?v=vid1").unwrap();
        assert_eq!(id.as_str(), "vid1");
    }

    #[test]
    fn it_should_parse_a_full_youtube_watch_url_with_extra_query_params() {
        let id =
            VideoId::from_url_or_id("https://youtube.com/watch?list=PL1&v=vid1&t=30s").unwrap();
        assert_eq!(id.as_str(), "vid1");
    }

    #[test]
    fn it_should_parse_a_short_youtu_be_url() {
        let id = VideoId::from_url_or_id("https://youtu.be/vid1").unwrap();
        assert_eq!(id.as_str(), "vid1");
    }

    #[test]
    fn it_should_parse_a_short_youtu_be_url_with_a_query_string() {
        let id = VideoId::from_url_or_id("https://youtu.be/vid1?t=30").unwrap();
        assert_eq!(id.as_str(), "vid1");
    }

    #[test]
    fn it_should_reject_a_youtube_watch_url_missing_the_v_parameter() {
        assert!(VideoId::from_url_or_id("https://www.youtube.com/watch?list=PL1").is_err());
    }

    #[test]
    fn it_should_reject_an_unrecognized_url() {
        assert!(VideoId::from_url_or_id("https://example.com/video/vid1").is_err());
    }

    #[test]
    fn it_should_reject_an_empty_value_via_from_url_or_id() {
        assert!(VideoId::from_url_or_id("").is_err());
    }
}
