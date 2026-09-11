use super::errors::PlaylistError;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistKind {
    YoutubeLinked,
    Custom,
}

impl PlaylistKind {
    pub fn new(value: impl Into<String>) -> Result<Self, PlaylistError> {
        let value = value.into();
        match value.as_str() {
            "youtube_linked" => Ok(Self::YoutubeLinked),
            "custom" => Ok(Self::Custom),
            _ => Err(PlaylistError(format!(
                "Playlist kind must be one of \"youtube_linked\" or \"custom\" (got \"{value}\")"
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::YoutubeLinked => "youtube_linked",
            Self::Custom => "custom",
        }
    }
}

impl fmt::Display for PlaylistKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_parse_each_valid_kind_value() {
        assert_eq!(
            PlaylistKind::new("youtube_linked").unwrap(),
            PlaylistKind::YoutubeLinked
        );
        assert_eq!(PlaylistKind::new("custom").unwrap(), PlaylistKind::Custom);
    }

    #[test]
    fn it_should_reject_an_invalid_kind_value_with_a_meaningful_message() {
        let error = PlaylistKind::new("bogus").unwrap_err();

        assert_eq!(
            error.to_string(),
            "Playlist kind must be one of \"youtube_linked\" or \"custom\" (got \"bogus\")"
        );
    }
}
