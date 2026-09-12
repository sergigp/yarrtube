use super::errors::PlaylistIdError;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlaylistId(String);

impl PlaylistId {
    pub fn new(id: impl Into<String>) -> Result<Self, PlaylistIdError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(PlaylistIdError(
                "YouTube playlist ID must not be empty".to_string(),
            ));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parses a bare YouTube playlist ID or a full YouTube URL carrying a
    /// `list` query parameter (`youtube.com/playlist?list=...`,
    /// `youtube.com/watch?v=...&list=...`) into a `PlaylistId`. Anything else
    /// (an unrecognized URL, a recognized URL missing `list`, an empty value)
    /// is rejected.
    pub fn from_url_or_id(value: impl Into<String>) -> Result<Self, PlaylistIdError> {
        let value = value.into();
        let trimmed = value.trim();

        let Some(after_scheme) = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))
        else {
            if trimmed.is_empty() {
                return Err(PlaylistIdError(
                    "Playlist ID or URL must not be empty".to_string(),
                ));
            }
            return Self::new(trimmed);
        };

        let (host, rest) = after_scheme.split_once('/').unwrap_or((after_scheme, ""));

        if host != "youtube.com" && host != "www.youtube.com" && host != "m.youtube.com" {
            return Err(PlaylistIdError(format!(
                "\"{value}\" is not a recognized YouTube playlist URL"
            )));
        }

        let query = rest.split_once('?').map(|(_, query)| query).unwrap_or("");
        let id = query.split('&').find_map(|pair| pair.strip_prefix("list="));
        match id {
            Some(id) if !id.is_empty() => Self::new(id),
            _ => Err(PlaylistIdError(format!(
                "YouTube URL is missing a \"list\" query parameter (got \"{value}\")"
            ))),
        }
    }
}

impl fmt::Display for PlaylistId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PlaylistId {
    type Err = PlaylistIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_non_empty_id() {
        let id = PlaylistId::new("PLabc123").unwrap();
        assert_eq!(id.as_str(), "PLabc123");
    }

    #[test]
    fn it_should_reject_an_empty_id() {
        assert!(PlaylistId::new("").is_err());
        assert!(PlaylistId::new("   ").is_err());
    }

    #[test]
    fn it_should_accept_a_bare_id_via_from_url_or_id() {
        let id = PlaylistId::from_url_or_id("PLabc123").unwrap();
        assert_eq!(id.as_str(), "PLabc123");
    }

    #[test]
    fn it_should_parse_a_full_youtube_playlist_url() {
        let id =
            PlaylistId::from_url_or_id("https://www.youtube.com/playlist?list=PLabc123").unwrap();
        assert_eq!(id.as_str(), "PLabc123");
    }

    #[test]
    fn it_should_parse_a_youtube_watch_url_carrying_a_list_parameter() {
        let id =
            PlaylistId::from_url_or_id("https://youtube.com/watch?v=vid1&list=PLabc123").unwrap();
        assert_eq!(id.as_str(), "PLabc123");
    }

    #[test]
    fn it_should_parse_a_youtube_playlist_url_with_extra_query_params() {
        let id = PlaylistId::from_url_or_id(
            "https://m.youtube.com/playlist?list=PLabc123&index=2&feature=share",
        )
        .unwrap();
        assert_eq!(id.as_str(), "PLabc123");
    }

    #[test]
    fn it_should_reject_a_youtube_url_missing_the_list_parameter() {
        assert!(PlaylistId::from_url_or_id("https://www.youtube.com/watch?v=vid1").is_err());
    }

    #[test]
    fn it_should_reject_an_unrecognized_url() {
        assert!(PlaylistId::from_url_or_id("https://example.com/playlist?list=PLabc123").is_err());
    }

    #[test]
    fn it_should_reject_an_empty_value_via_from_url_or_id() {
        assert!(PlaylistId::from_url_or_id("").is_err());
        assert!(PlaylistId::from_url_or_id("   ").is_err());
    }
}
