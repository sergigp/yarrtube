use crate::domain::category::{Category, CategoryId, DeleteCategoryError};
use crate::domain::services::{VideoCategoryAssigner, VideoCategoryAssignerApi};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use std::sync::Arc;

/// Deletes categories. Their member sources and videos are left untouched;
/// only the videos' membership of the category goes.
#[derive(Clone)]
pub struct CategoryDeleter {
    repository: Arc<dyn CategoryRepository>,
    video_category_assigner: VideoCategoryAssigner,
}

impl CategoryDeleter {
    pub fn new(
        repository: Arc<dyn CategoryRepository>,
        video_category_assigner: VideoCategoryAssigner,
    ) -> Self {
        Self {
            repository,
            video_category_assigner,
        }
    }
}

pub trait CategoryDeleterApi: Send + Sync {
    fn delete(&self, id: &CategoryId) -> Result<(), DeleteCategoryError>;
}

impl CategoryDeleterApi for CategoryDeleter {
    fn delete(&self, id: &CategoryId) -> Result<(), DeleteCategoryError> {
        let category = self.find_category(id)?;
        self.uncategorize_videos(&category)?;
        self.repository
            .delete(id)
            .map_err(DeleteCategoryError::Repository)
    }
}

impl CategoryDeleter {
    fn find_category(&self, id: &CategoryId) -> Result<Category, DeleteCategoryError> {
        self.repository
            .find(id)
            .map_err(DeleteCategoryError::Repository)?
            .ok_or_else(|| DeleteCategoryError::NotFound(id.clone()))
    }

    /// Recomputes the categories of `category`'s videos as if it were
    /// already deleted, before deleting it: a failed delete is healed by its
    /// members' next reconcile pass.
    fn uncategorize_videos(&self, category: &Category) -> Result<(), DeleteCategoryError> {
        let remaining: Vec<Category> = self
            .repository
            .list()
            .map_err(DeleteCategoryError::Repository)?
            .into_iter()
            .filter(|other| other.id != category.id)
            .collect();
        self.video_category_assigner
            .reassign_category(&category.id, &category.members, &remaining)
            .map_err(DeleteCategoryError::Repository)
    }
}
