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
        id: &CategoryId,
        members: Vec<CategoryMember>,
    ) -> Result<(), UpdateCategoryError> {
        let category = self.find_category(id)?;
        self.ensure_tracked(&members)?;
        self.save(&category.with_members(members))
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

    /// Fails on the first of `members` that is not a tracked source.
    fn ensure_tracked(&self, members: &[CategoryMember]) -> Result<(), UpdateCategoryError> {
        members.iter().try_for_each(|member| {
            if self.is_tracked(member)? {
                Ok(())
            } else {
                Err(UpdateCategoryError::MemberNotTracked(member.clone()))
            }
        })
    }

    fn is_tracked(&self, member: &CategoryMember) -> Result<bool, UpdateCategoryError> {
        match member {
            CategoryMember::Channel(handle) => self
                .channel_repository
                .find(handle)
                .map(|channel| channel.is_some()),
            CategoryMember::Playlist(id) => self
                .playlist_repository
                .find(id)
                .map(|playlist| playlist.is_some()),
        }
        .map_err(UpdateCategoryError::Repository)
    }

    fn save(&self, category: &Category) -> Result<(), UpdateCategoryError> {
        self.repository
            .update(category)
            .map_err(UpdateCategoryError::Repository)
    }
}
