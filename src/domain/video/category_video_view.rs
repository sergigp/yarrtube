use super::sourced_video::{SourcedVideo, VideoSource};
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

impl From<SourcedVideo> for CategoryVideoView {
    fn from(sourced: SourcedVideo) -> Self {
        Self {
            video: sourced.video,
            metadata: None,
            source: sourced.source,
            category_ids: Vec::new(),
        }
    }
}

impl CategoryVideoView {
    pub fn with_metadata(self, metadata: Option<VideoMetadata>) -> Self {
        Self { metadata, ..self }
    }

    pub fn with_category_ids(self, category_ids: Vec<CategoryId>) -> Self {
        Self {
            category_ids,
            ..self
        }
    }
}
