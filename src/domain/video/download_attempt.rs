use super::downloaded_video::DownloadedVideo;

/// The outcome of a `download_video` attempt: either a `DownloadedVideo`, or
/// a clean `yt-dlp` failure (non-zero exit) carrying whatever text `yt-dlp`
/// wrote to its stderr, trimmed and `None` if it wrote nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadAttempt {
    Succeeded(DownloadedVideo),
    Failed {
        stderr: Option<String>,
        /// `Some(reason)` when `yt-dlp` reported the SABR-only streaming
        /// experiment while failing; see `DownloadedVideo::sabr_notice`.
        sabr_notice: Option<String>,
    },
}
