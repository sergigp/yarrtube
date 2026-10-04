use serde::Serialize;

/// A video's media file was downloaded into `folder`, inside its
/// owner's `output_dir`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoDownloaded {
    pub video_id: String,
    pub output_dir: String,
    pub folder: String,
}

impl VideoDownloaded {
    pub const EVENT_TYPE: &str = "video_downloaded";
}
