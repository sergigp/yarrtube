use crate::domain::category::{
    Category, CategoryColor, CategoryId, CategoryMember, CategoryName, UpdateCategoryError,
};
use crate::domain::services::{VideoCategoryAssigner, VideoCategoryAssignerApi};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use std::sync::Arc;

/// The fields of a category to change; absent ones are kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryUpdate {
    pub name: Option<CategoryName>,
    pub color: Option<CategoryColor>,
}

/// Renames and recolors categories and sets their members.
#[derive(Clone)]
pub struct CategoryUpdater {
    repository: Arc<dyn CategoryRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_category_assigner: VideoCategoryAssigner,
}

impl CategoryUpdater {
    pub fn new(
        repository: Arc<dyn CategoryRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_category_assigner: VideoCategoryAssigner,
    ) -> Self {
        Self {
            repository,
            channel_repository,
            playlist_repository,
            video_category_assigner,
        }
    }
}

pub trait CategoryUpdaterApi: Send + Sync {
    /// Applies the present fields of `update` in one save. Another category
    /// with the same name ignoring case → `NameTaken`. A rename is announced
    /// for every video in the category; a recolor is not.
    fn update(&self, id: &CategoryId, update: CategoryUpdate) -> Result<(), UpdateCategoryError>;
    /// Replaces the members, then recomputes the categories of every video of
    /// a former or new member. Every member must be a tracked
    /// channel/playlist; the first that is not → `MemberNotTracked`.
    fn set_members(
        &self,
        id: &CategoryId,
        members: Vec<CategoryMember>,
    ) -> Result<(), UpdateCategoryError>;
}

impl CategoryUpdaterApi for CategoryUpdater {
    fn update(&self, id: &CategoryId, update: CategoryUpdate) -> Result<(), UpdateCategoryError> {
        let category = self.find_category(id)?;
        if let Some(name) = &update.name {
            self.ensure_name_free(id, name)?;
        }
        let renamed = update.name.is_some();
        self.save(&Self::applied(category, update))?;
        if renamed {
            self.announce_renamed(id)?;
        }
        Ok(())
    }

    fn set_members(
        &self,
        id: &CategoryId,
        members: Vec<CategoryMember>,
    ) -> Result<(), UpdateCategoryError> {
        let category = self.find_category(id)?;
        self.ensure_tracked(&members)?;
        let previous_members = category.members.clone();
        self.save(&category.with_members(members))?;
        self.reassign_videos(id, &previous_members)
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

    /// `category` with the present fields of `update`.
    fn applied(category: Category, update: CategoryUpdate) -> Category {
        let category = match update.name {
            Some(name) => category.with_name(name),
            None => category,
        };
        match update.color {
            Some(color) => category.with_color(color),
            None => category,
        }
    }

    fn announce_renamed(&self, id: &CategoryId) -> Result<(), UpdateCategoryError> {
        self.video_category_assigner
            .announce_renamed(id)
            .map_err(UpdateCategoryError::Repository)
    }

    /// Recomputes the categories of every video `id`'s member change touches.
    fn reassign_videos(
        &self,
        id: &CategoryId,
        previous_members: &[CategoryMember],
    ) -> Result<(), UpdateCategoryError> {
        let categories = self
            .repository
            .list()
            .map_err(UpdateCategoryError::Repository)?;
        self.video_category_assigner
            .reassign_category(id, previous_members, &categories)
            .map_err(UpdateCategoryError::Repository)
    }

    fn save(&self, category: &Category) -> Result<(), UpdateCategoryError> {
        self.repository
            .update(category)
            .map_err(UpdateCategoryError::Repository)
    }
}
