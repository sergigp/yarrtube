use crate::domain::category::{
    Category, CategoryColor, CategoryMember, CategoryName, CategoryPrompt, CreateCategoryError,
};
use crate::domain::services::VideoCategoryAssigner;
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
    #[allow(dead_code)]
    channel_repository: Arc<dyn ChannelRepository>,
    #[allow(dead_code)]
    playlist_repository: Arc<dyn PlaylistRepository>,
    #[allow(dead_code)]
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
    /// Creates a category with no members, colored `color` or, without one,
    /// the least used palette color. Another category with the same name
    /// ignoring case → `NameTaken`.
    fn create(&self, category: NewCategory) -> Result<Category, CreateCategoryError>;
}

impl CategoryCreatorApi for CategoryCreator {
    fn create(&self, category: NewCategory) -> Result<Category, CreateCategoryError> {
        let existing = self.list_categories()?;
        Self::ensure_name_free(&existing, &category.name)?;
        let color = category
            .color
            .unwrap_or_else(|| Self::least_used_color(&existing));
        let category = Category::create(category.name, color, self.clock.now())
            .with_ai_prompt(category.ai_prompt)
            .with_members(category.members);
        self.repository
            .insert(&category)
            .map_err(CreateCategoryError::Repository)?;
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

    fn least_used_color(existing: &[Category]) -> CategoryColor {
        let used: Vec<CategoryColor> = existing
            .iter()
            .map(|category| category.color.clone())
            .collect();
        CategoryColor::least_used(&used)
    }
}
