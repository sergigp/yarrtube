use crate::domain::services::VideoOwnerFinder;
use crate::domain::video::VideoRecordId;
use crate::domain::video_category::{EditVideoCategoriesError, VideoCategoryChoice};
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
    /// Every category, name order, as offered for `video_id`.
    fn search(
        &self,
        video_id: &VideoRecordId,
    ) -> Result<Vec<VideoCategoryChoice>, EditVideoCategoriesError>;
}

impl VideoCategoryChoiceSearcherApi for VideoCategoryChoiceSearcher {
    fn search(
        &self,
        _video_id: &VideoRecordId,
    ) -> Result<Vec<VideoCategoryChoice>, EditVideoCategoriesError> {
        Ok(Vec::new())
    }
}
