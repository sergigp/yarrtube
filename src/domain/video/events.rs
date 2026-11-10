use super::episode_number::EpisodeNumber;
use super::video_id::VideoId;
use serde::Serialize;
use std::path::Path;

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

    /// The video's media file landed in `number`'s season folder inside
    /// `output_dir`, whether a first download or a quality upgrade put it
    /// there.
    pub fn new(video_id: &VideoId, output_dir: &Path, number: &EpisodeNumber) -> Self {
        Self {
            video_id: video_id.as_str().to_string(),
            output_dir: output_dir.to_string_lossy().to_string(),
            folder: number.season_folder(),
        }
    }
}

/// A video's subtitle file was placed in its owner's `output_dir`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoSubtitlesFetched {
    pub video_id: String,
    pub output_dir: String,
}

impl VideoSubtitlesFetched {
    pub const EVENT_TYPE: &str = "video_subtitles_fetched";
}
