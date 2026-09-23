use super::directory_path::DirectoryPath;
use std::fmt;

#[derive(Debug)]
pub enum ListDirectoriesError {
    NotFound(DirectoryPath),
    Repository(anyhow::Error),
}

impl fmt::Display for ListDirectoriesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(path) => {
                write!(f, "no directory \"{path}\" exists under the videos root")
            }
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ListDirectoriesError {}
