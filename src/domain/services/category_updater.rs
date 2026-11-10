use crate::domain::category::{
    Category, CategoryColor, CategoryId, CategoryMember, CategoryName, CategoryPrompt,
    UpdateCategoryError,
};
use crate::domain::services::{
    CategorizationResetter, CategorizationResetterApi, VideoCategoryAssigner,
    VideoCategoryAssignerApi,
};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::VideoCategoryVerdictRepository;
use std::sync::Arc;

/// The fields of a category to change; absent ones are kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryUpdate {
    pub name: Option<CategoryName>,
    pub color: Option<CategoryColor>,
    /// `Some(None)` removes the prompt.
    pub ai_prompt: Option<Option<CategoryPrompt>>,
}

/// Renames and recolors categories and sets their members.
#[derive(Clone)]
pub struct CategoryUpdater {
    repository: Arc<dyn CategoryRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_category_assigner: VideoCategoryAssigner,
    video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
    categorization_resetter: CategorizationResetter,
}

impl CategoryUpdater {
    pub fn new(
        repository: Arc<dyn CategoryRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_category_assigner: VideoCategoryAssigner,
        video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
        categorization_resetter: CategorizationResetter,
    ) -> Self {
        Self {
            repository,
            channel_repository,
            playlist_repository,
            video_category_assigner,
            video_category_verdict_repository,
            categorization_resetter,
        }
    }
}

pub trait CategoryUpdaterApi: Send + Sync {
    /// Applies the present fields of `update` in one save. Another category
    /// with the same name ignoring case → `NameTaken`. A rename is announced
    /// for every video in the category; a recolor is not. A changed AI prompt also deletes the
    /// category's unavailable verdicts, so those videos are asked again; a
    /// first AI prompt makes the category pending for every stored video
    /// outside its members, whose categorization is Pending again.
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
        let reprompted = Self::changes_prompt(&category, &update);
        let first_prompt = category.ai_prompt.is_none() && reprompted;
        let updated = Self::applied(category, update);
        self.save(&updated)?;
        if reprompted {
            self.reset_failed_categorizations(id)?;
        }
        if first_prompt {
            self.reset_pending_categorizations(&updated)?;
        }
        if renamed {
            self.video_category_assigner
                .announce_renamed(id)
                .map_err(UpdateCategoryError::Repository)?;
        }
        Ok(())
    }

    fn set_members(
        &self,
        id: &CategoryId,
        members: Vec<CategoryMember>,
    ) -> Result<(), UpdateCategoryError> {
        let _guard = self.video_category_assigner.lock();
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
        let category = match update.color {
            Some(color) => category.with_color(color),
            None => category,
        };
        match update.ai_prompt {
            Some(ai_prompt) => category.with_ai_prompt(ai_prompt),
            None => category,
        }
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

    /// Whether `update` gives `category` a different AI prompt.
    fn changes_prompt(category: &Category, update: &CategoryUpdate) -> bool {
        update
            .ai_prompt
            .as_ref()
            .is_some_and(|prompt| prompt != &category.ai_prompt)
    }

    /// A new prompt may get answers the old one couldn't: the category's
    /// `failed` verdicts go, so it is pending again for their videos, whose
    /// categorization is Pending again. Under the lock, so a running
    /// categorization giving up either sees the new prompt or has its failed
    /// verdicts deleted here.
    fn reset_failed_categorizations(&self, id: &CategoryId) -> Result<(), UpdateCategoryError> {
        let _guard = self.video_category_assigner.lock();
        self.video_category_verdict_repository
            .delete_failed_for_category(id)
            .and_then(|video_ids| self.categorization_resetter.reset(&video_ids))
            .map_err(UpdateCategoryError::Repository)
    }

    /// Given its first AI prompt, `category` is pending for every stored
    /// video outside its members, whose categorization is Pending again.
    /// Under the lock, as `reset_failed_categorizations`.
    fn reset_pending_categorizations(
        &self,
        category: &Category,
    ) -> Result<(), UpdateCategoryError> {
        let _guard = self.video_category_assigner.lock();
        self.categorization_resetter
            .reset_pending_for(category)
            .map_err(UpdateCategoryError::Repository)
    }

    fn save(&self, category: &Category) -> Result<(), UpdateCategoryError> {
        self.repository
            .update(category)
            .map_err(UpdateCategoryError::Repository)
    }
}
