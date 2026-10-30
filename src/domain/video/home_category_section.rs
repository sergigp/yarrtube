use super::home_video_view::HomeVideoView;
use crate::domain::category::{CategoryColor, CategoryId, CategoryName};

/// A "More about <name>" home section: a category and its pending videos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeCategorySection {
    pub id: CategoryId,
    pub name: CategoryName,
    pub color: CategoryColor,
    pub videos: Vec<HomeVideoView>,
}
