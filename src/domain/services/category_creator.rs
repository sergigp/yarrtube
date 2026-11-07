use crate::domain::category::{
    Category, CategoryColor, CategoryMember, CategoryName, CategoryPrompt, CreateCategoryError,
};
use crate::domain::services::{VideoCategoryAssigner, VideoCategoryAssignerApi};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// A category to create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCategory {
    pub name: CategoryName,
    /// `None` takes the least used palette color.
    pub color: Option<CategoryColor>,
    pub ai_prompt: Option<CategoryPrompt>,
    pub members: Vec<CategoryMember>,
}

/// Creates categories, each with a name no other category has.
#[derive(Clone)]
pub struct CategoryCreator {
    repository: Arc<dyn CategoryRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_category_assigner: VideoCategoryAssigner,
    clock: Arc<dyn Clock>,
}

impl CategoryCreator {
    pub fn new(
        repository: Arc<dyn CategoryRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_category_assigner: VideoCategoryAssigner,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repository,
            channel_repository,
            playlist_repository,
            video_category_assigner,
            clock,
        }
    }
}

pub trait CategoryCreatorApi: Send + Sync {
    /// Creates `category` with its prompt and members in one save, then
    /// computes the categories of its members' stored videos. Another category
    /// with the same name ignoring case → `NameTaken`. Every member must be a
    /// tracked channel/playlist; the first that is not → `MemberNotTracked`.
    fn create(&self, category: NewCategory) -> Result<Category, CreateCategoryError>;
}

impl CategoryCreatorApi for CategoryCreator {
    fn create(&self, category: NewCategory) -> Result<Category, CreateCategoryError> {
        let _guard = self.video_category_assigner.lock();
        let existing = self.list_categories()?;
        Self::ensure_name_free(&existing, &category.name)?;
        self.ensure_tracked(&category.members)?;
        let color = category
            .color
            .unwrap_or_else(|| Self::least_used_color(&existing));
        let category = Category::create(category.name, color, self.clock.now())
            .with_ai_prompt(category.ai_prompt)
            .with_members(category.members);
        self.repository
            .insert(&category)
            .map_err(CreateCategoryError::Repository)?;
        self.assign_videos(&category)?;
        Ok(category)
    }
}

impl CategoryCreator {
    fn list_categories(&self) -> Result<Vec<Category>, CreateCategoryError> {
        self.repository
            .list()
            .map_err(CreateCategoryError::Repository)
    }

    fn ensure_name_free(
        existing: &[Category],
        name: &CategoryName,
    ) -> Result<(), CreateCategoryError> {
        if existing
            .iter()
            .any(|category| category.name.is_same_as(name))
        {
            return Err(CreateCategoryError::NameTaken(name.clone()));
        }
        Ok(())
    }

    /// Fails on the first of `members` that is not a tracked source.
    fn ensure_tracked(&self, members: &[CategoryMember]) -> Result<(), CreateCategoryError> {
        members.iter().try_for_each(|member| {
            if self.is_tracked(member)? {
                Ok(())
            } else {
                Err(CreateCategoryError::MemberNotTracked(member.clone()))
            }
        })
    }

    fn is_tracked(&self, member: &CategoryMember) -> Result<bool, CreateCategoryError> {
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
        .map_err(CreateCategoryError::Repository)
    }

    /// Puts the stored videos of `category`'s members in it.
    fn assign_videos(&self, category: &Category) -> Result<(), CreateCategoryError> {
        let categories = self.list_categories()?;
        self.video_category_assigner
            .reassign_category(&category.id, &[], &categories)
            .map_err(CreateCategoryError::Repository)
    }

    fn least_used_color(existing: &[Category]) -> CategoryColor {
        let used: Vec<CategoryColor> = existing
            .iter()
            .map(|category| category.color.clone())
            .collect();
        CategoryColor::least_used(&used)
    }
}
