use crate::domain::shared::Quality;
use crate::infrastructure::shared::ytdlp;
use std::path::Path;

/// Downloads a single video via `yt-dlp`, injected into `VideoService` for
/// the event-driven download path.
pub trait VideoDownloaderRepository: Send + Sync {
    /// Returns `Ok(Some(filename))` with the exact filename `yt-dlp` saved
    /// on a successful download, `Ok(None)` for a clean `yt-dlp` failure
    /// (non-zero exit). Returns `Err` only for a systemic problem (e.g.
    /// `yt-dlp` missing from `PATH`, or an unparseable `--print` output).
    fn download(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        quality: Quality,
        output_dir: &Path,
    ) -> anyhow::Result<Option<String>>;
}

pub struct YtDlpVideoDownloaderRepository;

impl VideoDownloaderRepository for YtDlpVideoDownloaderRepository {
    fn download(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        quality: Quality,
        output_dir: &Path,
    ) -> anyhow::Result<Option<String>> {
        ytdlp::ensure_output_dir(output_dir)?;
        ytdlp::download_video(video_url, desired_filename, video_id, quality, output_dir)
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeVideoDownloaderRepository {
    pub(crate) result: std::sync::Mutex<Option<String>>,
    #[allow(clippy::type_complexity)]
    pub(crate) calls: std::sync::Mutex<Vec<(String, String, String, Quality, std::path::PathBuf)>>,
}

#[cfg(test)]
impl FakeVideoDownloaderRepository {
    pub fn new(succeeds: bool) -> Self {
        Self {
            result: std::sync::Mutex::new(succeeds.then(|| "fake-output.mp4".to_string())),
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[cfg(test)]
impl VideoDownloaderRepository for FakeVideoDownloaderRepository {
    fn download(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        quality: Quality,
        output_dir: &Path,
    ) -> anyhow::Result<Option<String>> {
        self.calls.lock().unwrap().push((
            video_url.to_string(),
            desired_filename.to_string(),
            video_id.to_string(),
            quality,
            output_dir.to_path_buf(),
        ));
        Ok(self.result.lock().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::ytdlp::test_support;

    #[test]
    #[cfg(unix)]
    fn it_should_map_a_successful_yt_dlp_process_to_the_printed_filename() {
        let _guard = test_support::FakeYtDlpOnPath::with_exit_code(0);
        let output_dir = test_support::unique_temp_dir("video-downloader-repository");

        let result = YtDlpVideoDownloaderRepository
            .download(
                "https://example.com/video",
                "My Video",
                "vid1",
                Quality::High,
                &output_dir,
            )
            .unwrap();

        assert_eq!(
            result,
            Some(test_support::DEFAULT_PRINTED_FILENAME.to_string())
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_map_a_failed_yt_dlp_process_to_none() {
        let _guard = test_support::FakeYtDlpOnPath::with_exit_code(1);
        let output_dir = test_support::unique_temp_dir("video-downloader-repository");

        let result = YtDlpVideoDownloaderRepository
            .download(
                "https://example.com/video",
                "My Video",
                "vid1",
                Quality::High,
                &output_dir,
            )
            .unwrap();

        assert_eq!(result, None);
        std::fs::remove_dir_all(&output_dir).unwrap();
    }
}
