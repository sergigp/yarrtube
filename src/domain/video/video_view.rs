use super::video::Video;
use crate::domain::video_metadata::VideoMetadata;
use crate::domain::video_summary::VideoSummary;

/// A video as listed, with its generated metadata when it has any and its AI
/// summary when it has one that may be shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoView {
    pub video: Video,
    pub metadata: Option<VideoMetadata>,
    pub summary: Option<VideoSummary>,
}
