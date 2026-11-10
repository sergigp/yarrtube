use super::download_failure::DownloadFailure;
use super::downloaded_video::DownloadedVideo;

/// The outcome of a `download_video` attempt: either a `DownloadedVideo`, or
/// a clean `yt-dlp` failure (non-zero exit) carrying its `DownloadFailure`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadAttempt {
    Succeeded(DownloadedVideo),
    Failed {
        failure: DownloadFailure,
        /// `Some(reason)` when `yt-dlp` reported the SABR-only streaming
        /// experiment while failing; see `DownloadedVideo::sabr_notice`.
        sabr_notice: Option<String>,
    },
}
