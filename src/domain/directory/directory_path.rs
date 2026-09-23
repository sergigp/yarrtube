use super::errors::DirectoryPathError;
use std::fmt;

/// A path relative to the configured videos root, the empty path meaning the
/// root itself. Separate from `PlaylistPath`, which forbids the empty path a
/// browse of the root needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryPath(String);

impl DirectoryPath {
    pub fn new(path: impl Into<String>) -> Result<Self, DirectoryPathError> {
        let path = path.into();
        if path.trim().is_empty() {
            return Ok(Self::root());
        }
        if path.starts_with('/') {
            return Err(DirectoryPathError(
                "Directory path must not be an absolute path".to_string(),
            ));
        }

        for segment in path.split('/') {
            if segment.is_empty() {
                return Err(DirectoryPathError(
                    "Directory path must not contain empty segments".to_string(),
                ));
            }
            if segment == ".." {
                return Err(DirectoryPathError(
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
        let path = DirectoryPath::new("playlists").unwrap();

        assert_eq!(path.as_str(), "playlists");
        assert!(!path.is_root());
    }

    #[test]
    fn it_should_accept_a_multi_segment_path() {
        let path = DirectoryPath::new("playlists/music/chill").unwrap();

        assert_eq!(path.as_str(), "playlists/music/chill");
    }

    #[test]
    fn it_should_be_root_on_an_empty_path() {
        assert!(DirectoryPath::new("").unwrap().is_root());
        assert!(DirectoryPath::new("   ").unwrap().is_root());
        assert!(DirectoryPath::root().is_root());
        assert_eq!(DirectoryPath::root().as_str(), "");
    }

    #[test]
    fn it_should_reject_an_absolute_path() {
        assert!(DirectoryPath::new("/etc").is_err());
    }

    #[test]
    fn it_should_reject_a_path_containing_a_parent_traversal_segment() {
        assert!(DirectoryPath::new("playlists/../..").is_err());
        assert!(DirectoryPath::new("..").is_err());
    }

    #[test]
    fn it_should_reject_a_path_with_an_empty_segment() {
        assert!(DirectoryPath::new("playlists//music").is_err());
        assert!(DirectoryPath::new("playlists/music/").is_err());
    }
}
