use crate::domain::shared::Quality;
use crate::infrastructure::shared::ytdlp;
pub use crate::infrastructure::shared::ytdlp::{
    DownloadAttempt, DownloadedVideo, FetchedThumbnail,
};
use std::path::{Path, PathBuf};

/// Downloads a single video (or just its thumbnail) via `yt-dlp`, injected
/// into `VideoDownloader`/`ThumbnailFetcher` for the event-driven download
/// path.
pub trait VideoDownloaderRepository: Send + Sync {
    /// Returns `Ok(DownloadAttempt::Succeeded(..))` with the exact filename
    /// `yt-dlp` saved (and its duration, when known) on a successful
    /// download, `Ok(DownloadAttempt::Failed { stderr })` for a clean
    /// `yt-dlp` failure (non-zero exit), carrying `yt-dlp`'s reported error
    /// text when it reported one. Returns `Err` only for a systemic problem
    /// (e.g. `yt-dlp` missing from `PATH`, or an unparseable `--print`
    /// output). `existing_folder`, when `Some`, is reused verbatim as the
    /// video's per-video output folder instead of resolving a fresh one —
    /// see `video-download`'s "Video's thumbnail was already fetched ahead
    /// of its download" scenario.
    fn download(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        quality: Quality,
        output_dir: &Path,
        existing_folder: Option<&str>,
    ) -> anyhow::Result<DownloadAttempt>;

    /// Returns `Ok(Some(FetchedThumbnail))` when a thumbnail was fetched,
    /// `Ok(None)` when the video has none to fetch (not an error — see
    /// `video-thumbnails`). Returns `Err` only for a systemic problem.
    /// `existing_folder` is reused the same way as `download`'s — e.g. a
    /// `Downloaded` video's already-recorded folder, for a missing-thumbnail
    /// recovery pass.
    fn fetch_thumbnail(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        output_dir: &Path,
        existing_folder: Option<&str>,
    ) -> anyhow::Result<Option<FetchedThumbnail>>;

    /// Runs a simulate-only probe (forcing the alternate player clients) to
    /// reveal the *precise* reason a download failed, since `yt-dlp`'s default
    /// clients collapse many permanent blocks into a bare "Video unavailable".
    /// Returns `Ok(Some(reason))` with the reason `yt-dlp` printed,
    /// `Ok(None)` when none can be determined (clean exit, or only a bare
    /// "Video unavailable"), and `Err` only for a systemic problem. The probe
    /// never downloads — it only diagnoses.
    fn diagnose(&self, video_url: &str) -> anyhow::Result<Option<String>>;
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
        existing_folder: Option<&str>,
    ) -> anyhow::Result<DownloadAttempt> {
        ytdlp::ensure_output_dir(output_dir)?;
        ytdlp::download_video(
            &self.ytdlp_path,
            video_url,
            desired_filename,
            video_id,
            quality,
            output_dir,
            existing_folder,
        )
    }

    fn fetch_thumbnail(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        output_dir: &Path,
        existing_folder: Option<&str>,
    ) -> anyhow::Result<Option<FetchedThumbnail>> {
        ytdlp::ensure_output_dir(output_dir)?;
        ytdlp::fetch_thumbnail(
            &self.ytdlp_path,
            video_url,
            desired_filename,
            video_id,
            output_dir,
            existing_folder,
        )
    }

    fn diagnose(&self, video_url: &str) -> anyhow::Result<Option<String>> {
        ytdlp::diagnose(&self.ytdlp_path, video_url)
    }
}

/// What a `FakeVideoDownloaderRepository`'s `diagnose` yields: a determined
/// reason (`Some`), nothing determinable (`None`), or a systemic probe error
/// the downloader must swallow.
#[cfg(test)]
#[derive(Clone)]
pub(crate) enum DiagnoseOutcome {
    Reason(Option<String>),
    Error,
}

#[cfg(test)]
pub struct FakeVideoDownloaderRepository {
    pub(crate) result: std::sync::Mutex<DownloadAttempt>,
    pub(crate) diagnose_result: std::sync::Mutex<DiagnoseOutcome>,
    #[allow(clippy::type_complexity)]
    pub(crate) calls: std::sync::Mutex<
        Vec<(
            String,
            String,
            String,
            Quality,
            std::path::PathBuf,
            Option<String>,
        )>,
    >,
    #[allow(clippy::type_complexity)]
    pub(crate) thumbnail_result: std::sync::Mutex<Option<anyhow::Result<Option<FetchedThumbnail>>>>,
    #[allow(clippy::type_complexity)]
    pub(crate) thumbnail_calls:
        std::sync::Mutex<Vec<(String, String, String, std::path::PathBuf, Option<String>)>>,
    /// Runs while `download` is in flight, e.g. to delete the video
    /// mid-download.
    #[allow(clippy::type_complexity)]
    pub(crate) on_download: Option<Box<dyn Fn() + Send + Sync>>,
}

#[cfg(test)]
impl Default for FakeVideoDownloaderRepository {
    fn default() -> Self {
        Self::with_result(DownloadAttempt::Failed {
            stderr: None,
            sabr_notice: None,
        })
    }
}

#[cfg(test)]
impl FakeVideoDownloaderRepository {
    pub fn new(succeeds: bool) -> Self {
        if succeeds {
            Self::with_result(DownloadAttempt::Succeeded(DownloadedVideo {
                folder: "fake-output".to_string(),
                filename: "fake-output.mp4".to_string(),
                duration_seconds: None,
                sabr_notice: None,
            }))
        } else {
            Self::with_result(DownloadAttempt::Failed {
                stderr: None,
                sabr_notice: None,
            })
        }
    }

    /// Succeeds with `duration_seconds` recorded alongside the fake filename.
    pub fn with_duration(duration_seconds: i64) -> Self {
        Self::with_result(DownloadAttempt::Succeeded(DownloadedVideo {
            folder: "fake-output".to_string(),
            filename: "fake-output.mp4".to_string(),
            duration_seconds: Some(duration_seconds),
            sabr_notice: None,
        }))
    }

    /// Fails the download, carrying `stderr` as the reported `yt-dlp`
    /// error text.
    pub fn with_failed_stderr(stderr: &str) -> Self {
        Self::with_result(DownloadAttempt::Failed {
            stderr: Some(stderr.to_string()),
            sabr_notice: None,
        })
    }

    /// Succeeds but reports the SABR-only streaming experiment (a degraded,
    /// lower-quality fallback), so tests can drive the download path's
    /// SABR-on-success behavior.
    pub fn succeeding_with_sabr(reason: &str) -> Self {
        Self::with_result(DownloadAttempt::Succeeded(DownloadedVideo {
            folder: "fake-output".to_string(),
            filename: "fake-output.mp4".to_string(),
            duration_seconds: None,
            sabr_notice: Some(reason.to_string()),
        }))
    }

    /// Fails the download, carrying both `stderr` and a SABR-only notice, so
    /// tests can drive the download path's SABR-on-failure behavior.
    pub fn with_failed_stderr_and_sabr(stderr: &str, reason: &str) -> Self {
        Self::with_result(DownloadAttempt::Failed {
            stderr: Some(stderr.to_string()),
            sabr_notice: Some(reason.to_string()),
        })
    }

    /// Configures the result `fetch_thumbnail` returns; `None` (the
    /// default) mirrors a clean "no thumbnail available" outcome.
    pub fn with_thumbnail_result(self, result: Option<FetchedThumbnail>) -> Self {
        Self {
            thumbnail_result: std::sync::Mutex::new(Some(Ok(result))),
            ..self
        }
    }

    /// Makes `fetch_thumbnail` return `Err` instead of a configured result.
    pub fn with_thumbnail_error(self) -> Self {
        Self {
            thumbnail_result: std::sync::Mutex::new(Some(Err(anyhow::anyhow!(
                "fake thumbnail fetch error"
            )))),
            ..self
        }
    }

    /// Runs `hook` while `download` is in flight.
    pub fn with_on_download(self, hook: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            on_download: Some(Box::new(hook)),
            ..self
        }
    }

    /// Makes the diagnostic probe report `reason` as the precise failure
    /// reason.
    pub fn with_diagnosed_reason(self, reason: &str) -> Self {
        Self {
            diagnose_result: std::sync::Mutex::new(DiagnoseOutcome::Reason(Some(
                reason.to_string(),
            ))),
            ..self
        }
    }

    /// Makes the diagnostic probe itself fail, so tests can prove the
    /// downloader swallows it and treats the reason as undetermined.
    pub fn with_diagnose_error(self) -> Self {
        Self {
            diagnose_result: std::sync::Mutex::new(DiagnoseOutcome::Error),
            ..self
        }
    }

    fn with_result(result: DownloadAttempt) -> Self {
        Self {
            result: std::sync::Mutex::new(result),
            diagnose_result: std::sync::Mutex::new(DiagnoseOutcome::Reason(None)),
            calls: Default::default(),
            thumbnail_result: Default::default(),
            thumbnail_calls: Default::default(),
            on_download: None,
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
        existing_folder: Option<&str>,
    ) -> anyhow::Result<DownloadAttempt> {
        self.calls.lock().unwrap().push((
            video_url.to_string(),
            desired_filename.to_string(),
            video_id.to_string(),
            quality,
            output_dir.to_path_buf(),
            existing_folder.map(str::to_string),
        ));
        if let Some(hook) = &self.on_download {
            hook();
        }
        Ok(self.result.lock().unwrap().clone())
    }

    fn fetch_thumbnail(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        output_dir: &Path,
        existing_folder: Option<&str>,
    ) -> anyhow::Result<Option<FetchedThumbnail>> {
        self.thumbnail_calls.lock().unwrap().push((
            video_url.to_string(),
            desired_filename.to_string(),
            video_id.to_string(),
            output_dir.to_path_buf(),
            existing_folder.map(str::to_string),
        ));
        match self.thumbnail_result.lock().unwrap().take() {
            Some(result) => result,
            None => Ok(None),
        }
    }

    fn diagnose(&self, _video_url: &str) -> anyhow::Result<Option<String>> {
        match self.diagnose_result.lock().unwrap().clone() {
            DiagnoseOutcome::Reason(reason) => Ok(reason),
            DiagnoseOutcome::Error => Err(anyhow::anyhow!("fake diagnose error")),
        }
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
                None,
            )
            .unwrap();

        assert_eq!(
            result,
            DownloadAttempt::Succeeded(DownloadedVideo {
                folder: "My Video".to_string(),
                filename: test_support::DEFAULT_PRINTED_FILENAME.to_string(),
                duration_seconds: None,
                sabr_notice: None,
            })
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_map_a_failed_yt_dlp_process_to_a_failed_attempt_with_its_stderr() {
        use std::os::unix::fs::PermissionsExt;

        let output_dir = test_support::unique_temp_dir("video-downloader-repository");
        let bin_dir = test_support::unique_temp_dir("video-downloader-repository-fake-bin");
        let script_path = bin_dir.join("yt-dlp");
        std::fs::write(
            &script_path,
            "#!/bin/sh\nprintf '%s\\n' 'HTTP Error 403: Forbidden' >&2\nexit 1\n",
        )
        .unwrap();
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let result = YtDlpVideoDownloaderRepository::new(script_path)
            .download(
                "https://example.com/video",
                "My Video",
                "vid1",
                Quality::High,
                &output_dir,
                None,
            )
            .unwrap();

        assert_eq!(
            result,
            DownloadAttempt::Failed {
                stderr: Some("HTTP Error 403: Forbidden".to_string()),
                sabr_notice: None,
            }
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
        std::fs::remove_dir_all(&bin_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn it_should_map_a_successful_thumbnail_fetch_to_the_printed_filename() {
        let fake = test_support::FakeYtDlp::with_stdout("My Video.jpg\n");
        let output_dir = test_support::unique_temp_dir("video-downloader-repository-thumbnail");

        let result = YtDlpVideoDownloaderRepository::new(fake.path.clone())
            .fetch_thumbnail(
                "https://example.com/video",
                "My Video",
                "vid1",
                &output_dir,
                None,
            )
            .unwrap();

        assert_eq!(
            result,
            Some(FetchedThumbnail {
                folder: "My Video".to_string(),
                filename: "My Video.jpg".to_string(),
            })
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }
}
