use crate::domain::shared::ValidationError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistName(String);

const FILESYSTEM_UNSAFE_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

impl PlaylistName {
    pub fn new(name: impl Into<String>) -> Result<Self, ValidationError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ValidationError(
                "Playlist name must not be empty".to_string(),
            ));
        }
        if name.contains(FILESYSTEM_UNSAFE_CHARS) {
            return Err(ValidationError(format!(
                "Playlist name must not contain any of these characters: {}",
                FILESYSTEM_UNSAFE_CHARS.iter().collect::<String>()
            )));
        }
        Ok(Self(name))
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

    const UNSAFE_CHARACTERS_MESSAGE: &str =
        "Playlist name must not contain any of these characters: /\\:*?\"<>|";

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
    fn it_should_reject_a_name_with_a_slash() {
        assert_eq!(
            PlaylistName::new("a/b"),
            Err(error(UNSAFE_CHARACTERS_MESSAGE))
        );
    }

    #[test]
    fn it_should_reject_a_name_with_a_backslash() {
        assert_eq!(
            PlaylistName::new("a\\b"),
            Err(error(UNSAFE_CHARACTERS_MESSAGE))
        );
    }

    fn error(message: &str) -> ValidationError {
        ValidationError(message.to_string())
    }
}
