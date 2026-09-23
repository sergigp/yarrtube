use crate::domain::shared::ValidationError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistPath(String);

const FILESYSTEM_UNSAFE_CHARS: &[char] = &['\\', ':', '*', '?', '"', '<', '>', '|'];

impl PlaylistPath {
    pub fn new(path: impl Into<String>) -> Result<Self, ValidationError> {
        let path = path.into();
        if path.trim().is_empty() {
            return Err(ValidationError(
                "Playlist path must not be empty".to_string(),
            ));
        }
        if path.starts_with('/') {
            return Err(ValidationError(
                "Playlist path must not be an absolute path".to_string(),
            ));
        }

        for segment in path.split('/') {
            if segment.is_empty() {
                return Err(ValidationError(
                    "Playlist path must not contain empty segments".to_string(),
                ));
            }
            if segment == ".." {
                return Err(ValidationError(
                    "Playlist path must not contain \"..\" segments".to_string(),
                ));
            }
            if segment.contains(FILESYSTEM_UNSAFE_CHARS) {
                return Err(ValidationError(format!(
                    "Playlist path segments must not contain any of these characters: {}",
                    FILESYSTEM_UNSAFE_CHARS.iter().collect::<String>()
                )));
            }
        }

        Ok(Self(path))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlaylistPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNSAFE_CHARACTERS_MESSAGE: &str =
        "Playlist path segments must not contain any of these characters: \\:*?\"<>|";

    #[test]
    fn it_should_accept_a_single_segment_path() {
        assert_eq!(
            PlaylistPath::new("music"),
            Ok(PlaylistPath("music".to_string()))
        );
    }

    #[test]
    fn it_should_accept_a_multi_segment_path() {
        assert_eq!(
            PlaylistPath::new("a/b/c"),
            Ok(PlaylistPath("a/b/c".to_string()))
        );
    }

    #[test]
    fn it_should_reject_an_empty_path() {
        assert_eq!(
            PlaylistPath::new(""),
            Err(error("Playlist path must not be empty"))
        );
    }

    #[test]
    fn it_should_reject_a_blank_path() {
        assert_eq!(
            PlaylistPath::new("   "),
            Err(error("Playlist path must not be empty"))
        );
    }

    #[test]
    fn it_should_reject_an_absolute_path() {
        assert_eq!(
            PlaylistPath::new("/a/b"),
            Err(error("Playlist path must not be an absolute path"))
        );
    }

    #[test]
    fn it_should_reject_a_path_with_an_empty_segment() {
        assert_eq!(
            PlaylistPath::new("a//b"),
            Err(error("Playlist path must not contain empty segments"))
        );
    }

    #[test]
    fn it_should_reject_a_path_with_a_trailing_slash() {
        assert_eq!(
            PlaylistPath::new("a/b/"),
            Err(error("Playlist path must not contain empty segments"))
        );
    }

    #[test]
    fn it_should_reject_a_path_containing_a_parent_traversal_segment() {
        assert_eq!(
            PlaylistPath::new("a/../b"),
            Err(error("Playlist path must not contain \"..\" segments"))
        );
    }

    #[test]
    fn it_should_reject_a_path_that_is_a_parent_traversal_segment() {
        assert_eq!(
            PlaylistPath::new(".."),
            Err(error("Playlist path must not contain \"..\" segments"))
        );
    }

    #[test]
    fn it_should_reject_a_path_with_a_backslash() {
        assert_eq!(
            PlaylistPath::new("a\\b"),
            Err(error(UNSAFE_CHARACTERS_MESSAGE))
        );
    }

    #[test]
    fn it_should_reject_a_path_with_a_colon() {
        assert_eq!(
            PlaylistPath::new("a:b"),
            Err(error(UNSAFE_CHARACTERS_MESSAGE))
        );
    }

    #[test]
    fn it_should_reject_a_path_with_a_wildcard() {
        assert_eq!(
            PlaylistPath::new("music/*"),
            Err(error(UNSAFE_CHARACTERS_MESSAGE))
        );
    }

    fn error(message: &str) -> ValidationError {
        ValidationError(message.to_string())
    }
}
