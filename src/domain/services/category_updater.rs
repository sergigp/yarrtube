use crate::domain::category::{
    Category, CategoryId, CategoryMember, CategoryName, UpdateCategoryError,
};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use std::sync::Arc;

/// Renames categories and sets their members.
#[derive(Clone)]
pub struct CategoryUpdater {
    repository: Arc<dyn CategoryRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
}

impl CategoryUpdater {
    pub fn new(
        repository: Arc<dyn CategoryRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
    ) -> Self {
        Self {
            repository,
            channel_repository,
            playlist_repository,
        }
    }
}

pub trait CategoryUpdaterApi: Send + Sync {
    /// Another category with the same name ignoring case → `NameTaken`.
    fn rename(&self, id: &CategoryId, name: CategoryName) -> Result<(), UpdateCategoryError>;
    /// Replaces the members. Every member must be a tracked channel/playlist;
    /// the first that is not → `MemberNotTracked`.
    fn set_members(
        &self,
        id: &CategoryId,
        members: Vec<CategoryMember>,
    ) -> Result<(), UpdateCategoryError>;
}

impl CategoryUpdaterApi for CategoryUpdater {
    fn rename(&self, id: &CategoryId, name: CategoryName) -> Result<(), UpdateCategoryError> {
        let category = self.find_category(id)?;
        self.ensure_name_free(id, &name)?;
        self.save(&category.with_name(name))
    }

    fn set_members(
        &self,
        _id: &CategoryId,
        _members: Vec<CategoryMember>,
    ) -> Result<(), UpdateCategoryError> {
        Ok(())
    }
}

impl CategoryUpdater {
    fn find_category(&self, id: &CategoryId) -> Result<Category, UpdateCategoryError> {
        self.repository
            .find(id)
            .map_err(UpdateCategoryError::Repository)?
            .ok_or_else(|| UpdateCategoryError::NotFound(id.clone()))
    }

    /// Fails when a category other than `id` already has `name`.
    fn ensure_name_free(
        &self,
        id: &CategoryId,
        name: &CategoryName,
    ) -> Result<(), UpdateCategoryError> {
        let taken = self
            .repository
            .list()
            .map_err(UpdateCategoryError::Repository)?
            .iter()
            .any(|category| &category.id != id && category.name.is_same_as(name));
        if taken {
            return Err(UpdateCategoryError::NameTaken(name.clone()));
        }
        Ok(())
    }

    fn save(&self, category: &Category) -> Result<(), UpdateCategoryError> {
        self.repository
            .update(category)
            .map_err(UpdateCategoryError::Repository)
    }
}
