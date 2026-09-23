use crate::domain::shared::ValidationError;
use std::fmt;

/// A path relative to the configured videos root, the empty path meaning the
/// root itself. Separate from `PlaylistPath`, which forbids the empty path a
/// browse of the root needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryPath(String);

impl DirectoryPath {
    pub fn new(path: impl Into<String>) -> Result<Self, ValidationError> {
        let path = path.into();
        if path.trim().is_empty() {
            return Ok(Self::root());
        }
        if path.starts_with('/') {
            return Err(ValidationError(
                "Directory path must not be an absolute path".to_string(),
            ));
        }

        for segment in path.split('/') {
            if segment.is_empty() {
                return Err(ValidationError(
                    "Directory path must not contain empty segments".to_string(),
                ));
            }
            if segment == ".." {
                return Err(ValidationError(
                    "Directory path must not contain \"..\" segments".to_string(),
                ));
            }
        }

        Ok(Self(path))
    }

    pub fn root() -> Self {
        Self(String::new())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for DirectoryPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_a_single_segment_path() {
        assert_eq!(
            DirectoryPath::new("playlists"),
            Ok(DirectoryPath("playlists".to_string()))
        );
    }

    #[test]
    fn it_should_accept_a_multi_segment_path() {
        assert_eq!(
            DirectoryPath::new("playlists/music/chill"),
            Ok(DirectoryPath("playlists/music/chill".to_string()))
        );
    }

    #[test]
    fn it_should_be_root_on_an_empty_path() {
        assert_eq!(DirectoryPath::new(""), Ok(DirectoryPath::root()));
    }

    #[test]
    fn it_should_be_root_on_a_blank_path() {
        assert_eq!(DirectoryPath::new("   "), Ok(DirectoryPath::root()));
    }

    #[test]
    fn it_should_report_only_the_root_as_root() {
        assert!(DirectoryPath::root().is_root());
        assert!(!DirectoryPath("playlists".to_string()).is_root());
    }

    #[test]
    fn it_should_reject_an_absolute_path() {
        assert_eq!(
            DirectoryPath::new("/etc"),
            Err(error("Directory path must not be an absolute path"))
        );
    }

    #[test]
    fn it_should_reject_a_path_with_an_empty_segment() {
        assert_eq!(
            DirectoryPath::new("playlists//music"),
            Err(error("Directory path must not contain empty segments"))
        );
    }

    #[test]
    fn it_should_reject_a_path_with_a_trailing_slash() {
        assert_eq!(
            DirectoryPath::new("playlists/music/"),
            Err(error("Directory path must not contain empty segments"))
        );
    }

    #[test]
    fn it_should_reject_a_path_containing_a_parent_traversal_segment() {
        assert_eq!(
            DirectoryPath::new("playlists/../.."),
            Err(error("Directory path must not contain \"..\" segments"))
        );
    }

    #[test]
    fn it_should_reject_a_path_that_is_a_parent_traversal_segment() {
        assert_eq!(
            DirectoryPath::new(".."),
            Err(error("Directory path must not contain \"..\" segments"))
        );
    }

    fn error(message: &str) -> ValidationError {
        ValidationError(message.to_string())
    }
}
