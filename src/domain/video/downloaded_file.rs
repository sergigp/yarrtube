use super::video_output_entry::is_episode_path;
use crate::domain::shared::Quality;

/// What a finished download recorded: the video file, the quality tier it
/// was downloaded at and, when known, its duration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedFile {
    pub filename: String,
    pub quality: Quality,
    pub duration_seconds: Option<i64>,
}

impl DownloadedFile {
    /// Whether the file is in the per-video-folder (or flat) layout that
    /// predates the TV-show layout, i.e. not under a `Season ` folder.
    pub fn is_in_legacy_layout(&self) -> bool {
        !is_episode_path(&self.filename)
    }

    /// The file recorded at `filename` (a relocation).
    pub fn with_filename(self, filename: impl Into<String>) -> Self {
        Self {
            filename: filename.into(),
            ..self
        }
    }

    /// The file recorded at the `quality` tier (a quality upgrade).
    pub fn with_quality(self, quality: Quality) -> Self {
        Self { quality, ..self }
    }
}
