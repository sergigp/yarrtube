use crate::domain::services::{VideoOwnerFinder, VideoOwnerFinderApi};
use crate::domain::video::{VideoOwner, VideoRecordId};
use crate::domain::video_category::{
    EditVideoCategoriesError, VideoCategoryChoice, VideoCategoryChoices,
};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::VideoCategoryVerdictRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::sync::Arc;

/// Reads a video copy's categories with where each comes from.
#[derive(Clone)]
pub struct VideoCategoryChoiceSearcher {
    video_repository: Arc<dyn VideoRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    category_repository: Arc<dyn CategoryRepository>,
    video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
}

impl VideoCategoryChoiceSearcher {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        category_repository: Arc<dyn CategoryRepository>,
        video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
    ) -> Self {
        Self {
            video_repository,
            video_owner_finder,
            category_repository,
            video_category_verdict_repository,
        }
    }
}

pub trait VideoCategoryChoiceSearcherApi: Send + Sync {
    /// Every category, name order, as offered for `video_id`, and whether
    /// its source has categorization on.
    fn search(
        &self,
        video_id: &VideoRecordId,
    ) -> Result<VideoCategoryChoices, EditVideoCategoriesError>;
}

impl VideoCategoryChoiceSearcherApi for VideoCategoryChoiceSearcher {
    fn search(
        &self,
        video_id: &VideoRecordId,
    ) -> Result<VideoCategoryChoices, EditVideoCategoriesError> {
        let owner = self.find_owner(video_id)?;
        let categories = self
            .category_repository
            .list()
            .map_err(EditVideoCategoriesError::Repository)?;
        let verdicts = self
            .video_category_verdict_repository
            .find_many(std::slice::from_ref(video_id))
            .map_err(EditVideoCategoriesError::Repository)?;
        Ok(VideoCategoryChoices {
            choices: VideoCategoryChoice::list(&categories, &owner.source, &verdicts),
            categorization_enabled: owner.categorization_enabled,
        })
    }
}

impl VideoCategoryChoiceSearcher {
    /// The channel or playlist holding the stored `video_id`.
    fn find_owner(&self, video_id: &VideoRecordId) -> Result<VideoOwner, EditVideoCategoriesError> {
        let video = self
            .video_repository
            .find(video_id)
            .map_err(EditVideoCategoriesError::Repository)?
            .ok_or(EditVideoCategoriesError::VideoNotFound)?;
        self.video_owner_finder
            .find(&video)
            .map_err(EditVideoCategoriesError::Repository)?
            .ok_or(EditVideoCategoriesError::VideoNotFound)
    }
}
