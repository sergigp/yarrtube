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

/// A video's summary was stored and its summary step is Done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoSummaryGenerated {
    pub video_id: String,
}

impl VideoSummaryGenerated {
    pub const EVENT_TYPE: &str = "video_summary_generated";
}

/// A video with a recorded media or thumbnail file was discarded (see the
/// `video-discard` capability): its files in its owner's `output_dir` are to
/// be deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoDiscarded {
    pub video_id: String,
    pub output_dir: String,
    pub filename: Option<String>,
    pub thumbnail_filename: Option<String>,
    pub subtitle_filename: Option<String>,
}

impl VideoDiscarded {
    pub const EVENT_TYPE: &str = "video_discarded";
}

/// A video's files were deleted from its show folder `output_dir`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoFileDeleted {
    pub output_dir: String,
}

impl VideoFileDeleted {
    pub const EVENT_TYPE: &str = "video_file_deleted";
}
