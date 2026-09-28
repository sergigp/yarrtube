use super::video::Video;
use crate::domain::video_metadata::VideoMetadata;

/// A video as listed, with its generated metadata when it has any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoView {
    pub video: Video,
    pub metadata: Option<VideoMetadata>,
}
