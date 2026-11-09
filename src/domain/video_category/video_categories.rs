use crate::domain::category::CategoryId;
use crate::domain::video::VideoId;

/// The categories one stored video belongs to. Empty = in none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoCategories {
    pub video_id: VideoId,
    /// Sorted, each once.
    pub category_ids: Vec<CategoryId>,
}

impl VideoCategories {
    pub fn new(video_id: VideoId, mut category_ids: Vec<CategoryId>) -> Self {
        category_ids.sort();
        category_ids.dedup();
        Self {
            video_id,
            category_ids,
        }
    }

    pub fn none(video_id: VideoId) -> Self {
        Self {
            video_id,
            category_ids: Vec::new(),
        }
    }
}
