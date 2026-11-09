use super::caption_tracks::CaptionTracks;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// A caption probe's result: the tracks yt-dlp's info JSON lists, the
/// video's publish time when reported, and the info JSON itself, written to
/// the staging dir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionProbe {
    pub tracks: CaptionTracks,
    pub published_at: Option<DateTime<Utc>>,
    /// Reused by `fetch_subtitles` (`--load-info-json`): one extraction per
    /// video, fewer YouTube requests.
    pub info_json: PathBuf,
    /// yt-dlp's stderr `WARNING:` lines that can affect caption tracks:
    /// any of them makes a probe without a selectable track inconclusive
    /// (yt-dlp withholds tracks silently, e.g. without a PO token). Cache
    /// write failures and skipped-formats notices are dropped.
    pub warnings: Vec<String>,
}
