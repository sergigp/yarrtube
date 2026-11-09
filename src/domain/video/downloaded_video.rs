use super::video_chapter::VideoChapter;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// A video's `yt-dlp`-reported download result, staged outside its show:
/// the media file's path, the converted thumbnail's path when `yt-dlp` wrote
/// one, its duration in whole seconds and its publish time, when known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedVideo {
    pub media_path: PathBuf,
    pub thumbnail_path: Option<PathBuf>,
    pub duration_seconds: Option<i64>,
    pub published_at: Option<DateTime<Utc>>,
    /// `Some(reason)` when `yt-dlp` reported YouTube's SABR-only streaming
    /// experiment during this download — the better formats were skipped as
    /// missing a URL, so this success may be a lower-quality fallback. `None`
    /// when no SABR signal was reported. Carries the reason text for logging;
    /// it never affects the recorded status.
    pub sabr_notice: Option<String>,
    /// The chapters `yt-dlp` reported (`Some(vec![])` for none); `None` when
    /// the printed chapters could not be read.
    pub chapters: Option<Vec<VideoChapter>>,
    /// The downloaded file's height in pixels, when `yt-dlp` printed one.
    pub height: Option<u32>,
}
