use crate::domain::shared::Quality;
use crate::infrastructure::shared::ytdlp;
pub use crate::infrastructure::shared::ytdlp::DownloadedVideo;
use std::path::{Path, PathBuf};

/// Downloads a single video via `yt-dlp`, injected into `VideoDownloader` for
/// the event-driven download path.
pub trait VideoDownloaderRepository: Send + Sync {
    /// Returns `Ok(Some(DownloadedVideo))` with the exact filename `yt-dlp`
    /// saved (and its duration, when known) on a successful download,
    /// `Ok(None)` for a clean `yt-dlp` failure (non-zero exit). Returns
    /// `Err` only for a systemic problem (e.g. `yt-dlp` missing from
    /// `PATH`, or an unparseable `--print` output).
    fn download(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        quality: Quality,
        output_dir: &Path,
    ) -> anyhow::Result<Option<DownloadedVideo>>;
}

/// Invokes `yt-dlp` at `ytdlp_path`, the same configured path `update-ytdlp`
/// installs to — see design.md's "Move the default YTDLP_PATH..." decision.
pub struct YtDlpVideoDownloaderRepository {
    ytdlp_path: PathBuf,
}

impl YtDlpVideoDownloaderRepository {
    pub fn new(ytdlp_path: PathBuf) -> Self {
        Self { ytdlp_path }
    }
}

impl VideoDownloaderRepository for YtDlpVideoDownloaderRepository {
    fn download(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        quality: Quality,
        output_dir: &Path,
    ) -> anyhow::Result<Option<DownloadedVideo>> {
        ytdlp::ensure_output_dir(output_dir)?;
        ytdlp::download_video(
            &self.ytdlp_path,
            video_url,
            desired_filename,
            video_id,
            quality,
            output_dir,
        )
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeVideoDownloaderRepository {
    pub(crate) result: std::sync::Mutex<Option<DownloadedVideo>>,
    #[allow(clippy::type_complexity)]
    pub(crate) calls: std::sync::Mutex<Vec<(String, String, String, Quality, std::path::PathBuf)>>,
}

#[cfg(test)]
impl FakeVideoDownloaderRepository {
    pub fn new(succeeds: bool) -> Self {
        Self {
            result: std::sync::Mutex::new(succeeds.then(|| DownloadedVideo {
                folder: "fake-output".to_string(),
                filename: "fake-output.mp4".to_string(),
                duration_seconds: None,
            })),
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Succeeds with `duration_seconds` recorded alongside the fake filename.
    pub fn with_duration(duration_seconds: i64) -> Self {
        Self {
            result: std::sync::Mutex::new(Some(DownloadedVideo {
                folder: "fake-output".to_string(),
                filename: "fake-output.mp4".to_string(),
                duration_seconds: Some(duration_seconds),
            })),
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
    ) -> anyhow::Result<Option<DownloadedVideo>> {
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
        let fake = test_support::FakeYtDlp::with_exit_code(0);
        let output_dir = test_support::unique_temp_dir("video-downloader-repository");

        let result = YtDlpVideoDownloaderRepository::new(fake.path.clone())
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
            Some(DownloadedVideo {
                folder: "My Video".to_string(),
                filename: test_support::DEFAULT_PRINTED_FILENAME.to_string(),
                duration_seconds: None,
            })
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_map_a_failed_yt_dlp_process_to_none() {
        let fake = test_support::FakeYtDlp::with_exit_code(1);
        let output_dir = test_support::unique_temp_dir("video-downloader-repository");

        let result = YtDlpVideoDownloaderRepository::new(fake.path.clone())
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
