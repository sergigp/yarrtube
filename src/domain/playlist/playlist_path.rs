use super::errors::PlaylistError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistPath(String);

const FILESYSTEM_UNSAFE_CHARS: &[char] = &['\\', ':', '*', '?', '"', '<', '>', '|'];

impl PlaylistPath {
    pub fn new(path: impl Into<String>) -> Result<Self, PlaylistError> {
        let path = path.into();
        if path.trim().is_empty() {
            return Err(PlaylistError("Playlist path must not be empty".to_string()));
        }
        if path.starts_with('/') {
            return Err(PlaylistError(
                "Playlist path must not be an absolute path".to_string(),
            ));
        }

        for segment in path.split('/') {
            if segment.is_empty() {
                return Err(PlaylistError(
                    "Playlist path must not contain empty segments".to_string(),
                ));
            }
            if segment == ".." {
                return Err(PlaylistError(
                    "Playlist path must not contain \"..\" segments".to_string(),
                ));
            }
            if segment.contains(FILESYSTEM_UNSAFE_CHARS) {
                return Err(PlaylistError(format!(
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

    #[test]
    fn it_should_accept_a_valid_single_segment_path() {
        assert!(PlaylistPath::new("music").is_ok());
    }

    #[test]
    fn it_should_accept_a_valid_multi_segment_path() {
        let path = PlaylistPath::new("a/b/c").unwrap();
        assert_eq!(path.as_str(), "a/b/c");
    }

    #[test]
    fn it_should_reject_an_empty_path() {
        assert!(PlaylistPath::new("").is_err());
        assert!(PlaylistPath::new("   ").is_err());
    }

    #[test]
    fn it_should_reject_an_absolute_path() {
        assert!(PlaylistPath::new("/a/b").is_err());
    }

    #[test]
    fn it_should_reject_a_path_containing_a_parent_traversal_segment() {
        assert!(PlaylistPath::new("a/../b").is_err());
        assert!(PlaylistPath::new("..").is_err());
    }

    #[test]
    fn it_should_reject_a_path_with_an_empty_segment() {
        assert!(PlaylistPath::new("a//b").is_err());
        assert!(PlaylistPath::new("a/b/").is_err());
    }

    #[test]
    fn it_should_reject_a_path_with_filesystem_unsafe_characters() {
        assert!(PlaylistPath::new("a\\b").is_err());
        assert!(PlaylistPath::new("a:b").is_err());
    }
}
