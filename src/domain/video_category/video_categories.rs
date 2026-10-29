use crate::domain::category::CategoryId;
use crate::domain::video::VideoRecordId;

/// The categories one stored video record belongs to. Empty = in none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoCategories {
    pub video_id: VideoRecordId,
    /// Sorted, each once.
    pub category_ids: Vec<CategoryId>,
}

impl VideoCategories {
    pub fn new(video_id: VideoRecordId, category_ids: Vec<CategoryId>) -> Self {
        let mut category_ids = category_ids;
        category_ids.sort();
        category_ids.dedup();
        Self {
            video_id,
            category_ids,
        }
    }

    pub fn none(video_id: VideoRecordId) -> Self {
        Self {
            video_id,
            category_ids: Vec::new(),
        }
    }
}
