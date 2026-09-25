use crate::domain::directory::{Directory, DirectoryPath, ListDirectoriesError};
use crate::infrastructure::repositories::filesystem_directory_repository::DirectoryRepository;
use std::sync::Arc;

/// Reads the directories under the videos root, one level at a time.
#[derive(Clone)]
pub struct DirectorySearcher {
    repository: Arc<dyn DirectoryRepository>,
}

impl DirectorySearcher {
    pub fn new(repository: Arc<dyn DirectoryRepository>) -> Self {
        Self { repository }
    }
}

pub trait DirectorySearcherApi: Send + Sync {
    fn list(&self, path: &DirectoryPath) -> Result<Directory, ListDirectoriesError>;
}

impl DirectorySearcherApi for DirectorySearcher {
    fn list(&self, path: &DirectoryPath) -> Result<Directory, ListDirectoriesError> {
        match self.repository.list(path) {
            Ok(Some(directory)) => Ok(directory),
            Ok(None) => Err(ListDirectoriesError::NotFound(path.clone())),
            Err(e) => Err(ListDirectoriesError::Repository(e)),
        }
    }
}
