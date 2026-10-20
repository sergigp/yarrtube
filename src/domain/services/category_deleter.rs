use crate::domain::category::{CategoryId, DeleteCategoryError};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use std::sync::Arc;

/// Deletes categories. Their member sources and videos are left untouched.
#[derive(Clone)]
pub struct CategoryDeleter {
    repository: Arc<dyn CategoryRepository>,
}

impl CategoryDeleter {
    pub fn new(repository: Arc<dyn CategoryRepository>) -> Self {
        Self { repository }
    }
}

pub trait CategoryDeleterApi: Send + Sync {
    fn delete(&self, id: &CategoryId) -> Result<(), DeleteCategoryError>;
}

impl CategoryDeleterApi for CategoryDeleter {
    fn delete(&self, id: &CategoryId) -> Result<(), DeleteCategoryError> {
        self.ensure_exists(id)?;
        self.repository
            .delete(id)
            .map_err(DeleteCategoryError::Repository)
    }
}

impl CategoryDeleter {
    fn ensure_exists(&self, id: &CategoryId) -> Result<(), DeleteCategoryError> {
        self.repository
            .find(id)
            .map_err(DeleteCategoryError::Repository)?
            .map(|_| ())
            .ok_or_else(|| DeleteCategoryError::NotFound(id.clone()))
    }
}
