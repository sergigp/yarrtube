//! What the YouTube contract tests share. Those tests run the adapters
//! against the real `yt-dlp` and YouTube Data API, so they're `#[ignore]`d
//! (CI and the default `cargo test` skip them) and run by
//! `scripts/run-youtube-contract-tests.sh`. The fixtures are public videos,
//! playlists and channels that are Creative-Commons licensed or maintained by
//! the project, and unlikely to change.

use std::path::{Path, PathBuf};

/// "Boat in Island [UHD]", 14 s, CC BY, by "Free HD videos - no copyright".
pub(crate) const SHORT_VIDEO_ID: &str = "mqWUW6ypo6w";
/// "Big Buck Bunny", CC BY, Blender Foundation (@BlenderOfficial), with
/// uploaded `en` and `fr` subtitles.
pub(crate) const CAPTIONED_VIDEO_ID: &str = "YE7VzlLtp-4";
/// An id YouTube has no video for.
pub(crate) const MISSING_VIDEO_ID: &str = "aaaaaaaaaaa";
/// The project's "test" playlist (maintainer-owned), holding only Big Buck
/// Bunny.
pub(crate) const PLAYLIST_ID: &str = "PLXWRoRTUXjks";
/// "Free HD videos - no copyright": CC BY uploads, inactive since 2023.
pub(crate) const CHANNEL_HANDLE: &str = "@FreeHDvideosnocopyright";

pub(crate) fn video_url(id: &str) -> String {
    format!("https://www.youtube.com/watch?v={id}")
}

/// `YTDLP_PATH`, else `yt-dlp` on `PATH`.
pub(crate) fn ytdlp_path() -> PathBuf {
    PathBuf::from(std::env::var("YTDLP_PATH").unwrap_or_else(|_| "yt-dlp".to_string()))
}

/// The real YouTube Data API's root.
pub(crate) fn api_url() -> String {
    crate::serve::DEFAULT_YOUTUBE_API_URL.to_string()
}

/// `YOUTUBE_API_KEY`, which the contract tests can't run without.
pub(crate) fn api_key() -> String {
    std::env::var("YOUTUBE_API_KEY")
        .expect("YOUTUBE_API_KEY must be set (see scripts/run-youtube-contract-tests.sh)")
}

/// `(codec_type, codec_name)` of every stream in `media`, via `ffprobe`.
pub(crate) fn stream_codecs(media: &Path) -> Vec<(String, String)> {
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type,codec_name",
            "-of",
            "csv=p=0",
        ])
        .arg(media)
        .output()
        .expect("ffprobe must be installed (it comes with ffmpeg)");
    assert!(output.status.success(), "ffprobe failed on {media:?}");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once(','))
        .map(|(name, kind)| (kind.to_string(), name.to_string()))
        .collect()
}
