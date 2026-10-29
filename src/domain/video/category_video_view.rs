use super::sourced_video::VideoSource;
use super::video::Video;
use crate::domain::category::CategoryId;
use crate::domain::video_metadata::VideoMetadata;

/// A downloaded video of a category: as listed for its source, with its
/// generated metadata when it has any, the source it is listed from and the
/// categories having it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryVideoView {
    pub video: Video,
    pub metadata: Option<VideoMetadata>,
    pub source: VideoSource,
    /// Categories having the YouTube video, in category-name order.
    pub category_ids: Vec<CategoryId>,
}
