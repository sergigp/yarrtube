use crate::domain::shared::ValidationError;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistKind {
    YoutubeLinked,
}

impl PlaylistKind {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        match value.as_str() {
            "youtube_linked" => Ok(Self::YoutubeLinked),
            _ => Err(ValidationError(format!(
                "Playlist kind must be \"youtube_linked\" (got \"{value}\")"
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::YoutubeLinked => "youtube_linked",
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
    fn it_should_parse_youtube_linked() {
        assert_eq!(
            PlaylistKind::new("youtube_linked"),
            Ok(PlaylistKind::YoutubeLinked)
        );
    }

    #[test]
    fn it_should_reject_an_unknown_kind() {
        assert_eq!(
            PlaylistKind::new("bogus"),
            Err(ValidationError(
                "Playlist kind must be \"youtube_linked\" (got \"bogus\")".to_string()
            ))
        );
    }

    #[test]
    fn it_should_reject_the_removed_custom_kind() {
        assert_eq!(
            PlaylistKind::new("custom"),
            Err(ValidationError(
                "Playlist kind must be \"youtube_linked\" (got \"custom\")".to_string()
            ))
        );
    }
}
