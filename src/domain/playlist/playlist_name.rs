use super::playlist_id::PlaylistId;
use crate::domain::shared::ValidationError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistName(String);

impl PlaylistName {
    pub fn new(name: impl Into<String>) -> Result<Self, ValidationError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ValidationError(
                "Playlist name must not be empty".to_string(),
            ));
        }
        Ok(Self(name))
    }

    /// Names a playlist after the title YouTube reports for it, or after its
    /// ID when that title is blank.
    pub fn from_youtube_title(title: &str, id: &PlaylistId) -> Self {
        if title.trim().is_empty() {
            return Self(id.as_str().to_string());
        }
        Self(title.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlaylistName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_name() {
        assert_eq!(
            PlaylistName::new("My Favorite Songs"),
            Ok(PlaylistName("My Favorite Songs".to_string()))
        );
    }

    #[test]
    fn it_should_reject_an_empty_name() {
        assert_eq!(
            PlaylistName::new(""),
            Err(error("Playlist name must not be empty"))
        );
    }

    #[test]
    fn it_should_reject_a_blank_name() {
        assert_eq!(
            PlaylistName::new("   "),
            Err(error("Playlist name must not be empty"))
        );
    }

    #[test]
    fn it_should_accept_a_name_with_filesystem_unsafe_characters() {
        assert_eq!(
            PlaylistName::new("AC/DC: hits?"),
            Ok(PlaylistName("AC/DC: hits?".to_string()))
        );
    }

    fn error(message: &str) -> ValidationError {
        ValidationError(message.to_string())
    }
}
