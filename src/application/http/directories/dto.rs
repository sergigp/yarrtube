use crate::domain::directory::Directory;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ListDirectoriesQuery {
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DirectoryResponse {
    /// Absolute configured videos root. No other endpoint exposes it, and the
    /// add dialog's destination preview cannot be rendered without it.
    /// Reported on every listing rather than only some so a caller never has
    /// to order one request before another.
    pub root: String,
    pub path: String,
    pub entries: Vec<DirectoryEntryResponse>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct DirectoryEntryResponse {
    pub name: String,
}

impl DirectoryResponse {
    pub fn new(videos_root: &str, directory: Directory) -> Self {
        Self {
            root: videos_root.to_string(),
            path: directory.path.as_str().to_string(),
            entries: directory
                .subdirectories
                .into_iter()
                .map(|name| DirectoryEntryResponse { name })
                .collect(),
        }
    }
}
