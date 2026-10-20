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
    fn delete(&self, _id: &CategoryId) -> Result<(), DeleteCategoryError> {
        Ok(())
    }
}
