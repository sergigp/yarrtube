use super::errors::PlaylistError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistName(String);

const FILESYSTEM_UNSAFE_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

impl PlaylistName {
    pub fn new(name: impl Into<String>) -> Result<Self, PlaylistError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(PlaylistError("Playlist name must not be empty".to_string()));
        }
        if name.contains(FILESYSTEM_UNSAFE_CHARS) {
            return Err(PlaylistError(format!(
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

    #[test]
    fn it_should_accept_a_valid_name() {
        assert!(PlaylistName::new("My Favorite Songs").is_ok());
    }

    #[test]
    fn it_should_reject_an_empty_name() {
        assert!(PlaylistName::new("").is_err());
        assert!(PlaylistName::new("   ").is_err());
    }

    #[test]
    fn it_should_reject_a_name_with_filesystem_unsafe_characters() {
        assert!(PlaylistName::new("a/b").is_err());
        assert!(PlaylistName::new("a\\b").is_err());
    }
}
