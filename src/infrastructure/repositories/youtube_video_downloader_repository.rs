use crate::infrastructure::shared::ytdlp;
use std::path::Path;

/// Downloads a single video via `yt-dlp`, injected into `VideoService` for
/// the event-driven download path (distinct from the CLI-only
/// `YoutubeDownloaderClient`, which downloads a whole playlist at once).
pub trait VideoDownloaderRepository: Send + Sync {
    /// Returns `Ok(true)`/`Ok(false)` for a completed `yt-dlp` process based
    /// on its exit status. Returns `Err` only for a systemic problem (e.g.
    /// `yt-dlp` missing from `PATH`).
    fn download(&self, video_url: &str, output_dir: &Path) -> anyhow::Result<bool>;
}

pub struct YtDlpVideoDownloaderRepository;

impl VideoDownloaderRepository for YtDlpVideoDownloaderRepository {
    fn download(&self, video_url: &str, output_dir: &Path) -> anyhow::Result<bool> {
        ytdlp::ensure_output_dir(output_dir)?;
        ytdlp::download_video(video_url, output_dir)
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeVideoDownloaderRepository {
    pub(crate) succeeds: std::sync::atomic::AtomicBool,
    pub(crate) calls: std::sync::Mutex<Vec<(String, std::path::PathBuf)>>,
}

#[cfg(test)]
impl FakeVideoDownloaderRepository {
    pub fn new(succeeds: bool) -> Self {
        Self {
            succeeds: std::sync::atomic::AtomicBool::new(succeeds),
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[cfg(test)]
impl VideoDownloaderRepository for FakeVideoDownloaderRepository {
    fn download(&self, video_url: &str, output_dir: &Path) -> anyhow::Result<bool> {
        self.calls
            .lock()
            .unwrap()
            .push((video_url.to_string(), output_dir.to_path_buf()));
        Ok(self.succeeds.load(std::sync::atomic::Ordering::SeqCst))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::ytdlp::test_support;

    #[test]
    #[cfg(unix)]
    fn it_should_map_a_successful_yt_dlp_process_to_ok_true() {
        let _guard = test_support::FakeYtDlpOnPath::with_exit_code(0);
        let output_dir = test_support::unique_temp_dir("video-downloader-repository");

        let result = YtDlpVideoDownloaderRepository
            .download("https://example.com/video", &output_dir)
            .unwrap();

        assert!(result);
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_map_a_failed_yt_dlp_process_to_ok_false() {
        let _guard = test_support::FakeYtDlpOnPath::with_exit_code(1);
        let output_dir = test_support::unique_temp_dir("video-downloader-repository");

        let result = YtDlpVideoDownloaderRepository
            .download("https://example.com/video", &output_dir)
            .unwrap();

        assert!(!result);
        std::fs::remove_dir_all(&output_dir).unwrap();
    }
}
