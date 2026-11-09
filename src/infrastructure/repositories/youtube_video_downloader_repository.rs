use crate::domain::shared::Quality;
#[cfg(test)]
use crate::domain::video::DownloadedVideo;
use crate::domain::video::{DownloadAttempt, QualityProbe};
use crate::infrastructure::shared::ytdlp;
use std::path::{Path, PathBuf};

/// Downloads a single video via `yt-dlp`, injected into
/// `VideoDownloader`/`QualityUpgrader` for the event-driven download path.
pub trait VideoDownloaderRepository: Send + Sync {
    /// Downloads the video into `staging_dir` (created first). Returns
    /// `Ok(DownloadAttempt::Succeeded(..))` with the staged media path (and
    /// thumbnail, duration and publish time, when known) on a successful
    /// download, `Ok(DownloadAttempt::Failed { stderr })` for a clean
    /// `yt-dlp` failure (non-zero exit), carrying `yt-dlp`'s reported error
    /// text when it reported one. Returns `Err` only for a systemic problem
    /// (e.g. `yt-dlp` missing, or an unparseable `--print` output).
    fn download(
        &self,
        video_url: &str,
        quality: Quality,
        staging_dir: &Path,
    ) -> anyhow::Result<DownloadAttempt>;

    /// Runs a simulate-only probe (forcing the alternate player clients) to
    /// reveal the *precise* reason a download failed, since `yt-dlp`'s default
    /// clients collapse many permanent blocks into a bare "Video unavailable".
    /// Returns `Ok(Some(reason))` with the reason `yt-dlp` printed,
    /// `Ok(None)` when none can be determined (clean exit, or only a bare
    /// "Video unavailable"), and `Err` only for a systemic problem. The probe
    /// never downloads — it only diagnoses.
    fn diagnose(&self, video_url: &str) -> anyhow::Result<Option<String>>;

    /// The height `quality` would download now, and the video's publish
    /// time, downloading nothing. `Err` on any failure, clean or systemic.
    fn probe_quality(&self, video_url: &str, quality: Quality) -> anyhow::Result<QualityProbe>;
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
        quality: Quality,
        staging_dir: &Path,
    ) -> anyhow::Result<DownloadAttempt> {
        ytdlp::download_video(&self.ytdlp_path, video_url, quality, staging_dir)
    }

    fn diagnose(&self, video_url: &str) -> anyhow::Result<Option<String>> {
        ytdlp::diagnose(&self.ytdlp_path, video_url)
    }

    fn probe_quality(&self, video_url: &str, quality: Quality) -> anyhow::Result<QualityProbe> {
        ytdlp::probe_quality(&self.ytdlp_path, video_url, quality)
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
    pub(crate) calls: std::sync::Mutex<Vec<(String, Quality, std::path::PathBuf)>>,
    /// Runs while `download` is in flight, e.g. to delete the video
    /// mid-download.
    #[allow(clippy::type_complexity)]
    pub(crate) on_download: Option<Box<dyn Fn() + Send + Sync>>,
    /// What `probe_quality` returns; unconfigured, it fails.
    pub(crate) quality_probe: std::sync::Mutex<Option<anyhow::Result<QualityProbe>>>,
    /// The (url, quality) pairs `probe_quality` was called with.
    pub(crate) quality_calls: std::sync::Mutex<Vec<(String, Quality)>>,
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
                height: None,
                media_path: PathBuf::from("fake-output.mp4"),
                thumbnail_path: None,
                duration_seconds: None,
                published_at: None,
                sabr_notice: None,
                chapters: None,
            }))
        } else {
            Self::with_result(DownloadAttempt::Failed {
                stderr: None,
                sabr_notice: None,
            })
        }
    }

    /// Succeeds with `downloaded`, its paths relative to the staging dir.
    pub fn succeeding_with(downloaded: DownloadedVideo) -> Self {
        Self::with_result(DownloadAttempt::Succeeded(downloaded))
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
            height: None,
            media_path: PathBuf::from("fake-output.mp4"),
            thumbnail_path: None,
            duration_seconds: None,
            published_at: None,
            sabr_notice: Some(reason.to_string()),
            chapters: None,
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

    pub fn with_quality_probe(self, result: anyhow::Result<QualityProbe>) -> Self {
        Self {
            quality_probe: std::sync::Mutex::new(Some(result)),
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
            on_download: None,
            quality_probe: Default::default(),
            quality_calls: Default::default(),
        }
    }
}

#[cfg(test)]
impl VideoDownloaderRepository for FakeVideoDownloaderRepository {
    /// A successful result's paths are relative to the staging dir it was
    /// asked to download into, as `yt-dlp` would report them.
    fn download(
        &self,
        video_url: &str,
        quality: Quality,
        staging_dir: &Path,
    ) -> anyhow::Result<DownloadAttempt> {
        self.calls.lock().unwrap().push((
            video_url.to_string(),
            quality,
            staging_dir.to_path_buf(),
        ));
        if let Some(hook) = &self.on_download {
            hook();
        }
        Ok(match self.result.lock().unwrap().clone() {
            DownloadAttempt::Succeeded(downloaded) => DownloadAttempt::Succeeded(DownloadedVideo {
                media_path: staging_dir.join(&downloaded.media_path),
                thumbnail_path: downloaded.thumbnail_path.map(|path| staging_dir.join(path)),
                ..downloaded
            }),
            failed => failed,
        })
    }

    fn diagnose(&self, _video_url: &str) -> anyhow::Result<Option<String>> {
        match self.diagnose_result.lock().unwrap().clone() {
            DiagnoseOutcome::Reason(reason) => Ok(reason),
            DiagnoseOutcome::Error => Err(anyhow::anyhow!("fake diagnose error")),
        }
    }

    fn probe_quality(&self, video_url: &str, quality: Quality) -> anyhow::Result<QualityProbe> {
        self.quality_calls
            .lock()
            .unwrap()
            .push((video_url.to_string(), quality));
        self.quality_probe
            .lock()
            .unwrap()
            .take()
            .unwrap_or_else(|| Err(anyhow::anyhow!("no fake quality probe configured")))
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
            .download("https://example.com/video", Quality::High, &output_dir)
            .unwrap();

        assert_eq!(
            result,
            DownloadAttempt::Succeeded(DownloadedVideo {
                height: None,
                media_path: output_dir.join(test_support::DEFAULT_PRINTED_FILENAME),
                thumbnail_path: None,
                duration_seconds: None,
                published_at: None,
                sabr_notice: None,
                chapters: Some(vec![]),
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
            .download("https://example.com/video", Quality::High, &output_dir)
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
}

/// Against the real `yt-dlp` and YouTube: run with
/// `scripts/run-tests.sh --only contract` (see `shared::youtube_contract`).
#[cfg(test)]
mod youtube_contract_tests {
    use super::*;
    use crate::infrastructure::shared::youtube_contract::{
        CHAPTERED_VIDEO_ID, MISSING_VIDEO_ID, SHORT_VIDEO_ID, embedded_chapters, flamenco_chapters,
        stream_codecs, video_height, video_url, ytdlp_path,
    };
    use crate::infrastructure::shared::ytdlp::test_support::unique_temp_dir;
    use chrono::DateTime;

    #[test]
    #[ignore = "real YouTube: run scripts/run-tests.sh --only contract"]
    fn it_should_download_a_playable_mp4() {
        let staging_dir = unique_temp_dir("contract-download");
        let repository = YtDlpVideoDownloaderRepository::new(ytdlp_path());

        let attempt = repository
            .download(&video_url(SHORT_VIDEO_ID), Quality::Low, &staging_dir)
            .unwrap();

        // Whether YouTube runs its SABR streaming experiment on a download
        // varies run to run and only ever adds a notice, so it's taken as
        // reported.
        let sabr_notice = match &attempt {
            DownloadAttempt::Succeeded(video) => video.sabr_notice.clone(),
            DownloadAttempt::Failed { .. } => None,
        };
        let media_path = staging_dir.join(format!("{SHORT_VIDEO_ID}.mp4"));
        assert_eq!(
            attempt,
            DownloadAttempt::Succeeded(DownloadedVideo {
                height: Some(480),
                media_path: media_path.clone(),
                thumbnail_path: Some(staging_dir.join(format!("{SHORT_VIDEO_ID}.jpg"))),
                duration_seconds: Some(14),
                published_at: DateTime::from_timestamp(1_550_518_860, 0),
                sabr_notice,
                chapters: Some(vec![]),
            })
        );
        // H.264 + AAC, what every browser (iPhone Safari included) plays,
        // plus the thumbnail `--embed-thumbnail` adds as cover art.
        assert_eq!(
            stream_codecs(&media_path),
            vec![
                ("video".to_string(), "h264".to_string()),
                ("audio".to_string(), "aac".to_string()),
                ("video".to_string(), "mjpeg".to_string()),
            ]
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[ignore = "real YouTube: run scripts/run-tests.sh --only contract"]
    fn it_should_download_a_video_with_its_chapters_embedded() {
        let staging_dir = unique_temp_dir("contract-download-chapters");
        let repository = YtDlpVideoDownloaderRepository::new(ytdlp_path());

        let attempt = repository
            .download(&video_url(CHAPTERED_VIDEO_ID), Quality::Low, &staging_dir)
            .unwrap();

        let downloaded_chapters = match &attempt {
            DownloadAttempt::Succeeded(video) => video.chapters.clone(),
            DownloadAttempt::Failed { stderr, .. } => panic!("download failed: {stderr:?}"),
        };
        assert_eq!(downloaded_chapters, Some(flamenco_chapters()));
        assert_eq!(
            embedded_chapters(&staging_dir.join(format!("{CHAPTERED_VIDEO_ID}.mp4"))),
            flamenco_chapters()
                .into_iter()
                .map(|chapter| (chapter.start_seconds, chapter.title))
                .collect::<Vec<_>>()
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[ignore = "real YouTube: run scripts/run-tests.sh --only contract"]
    fn it_should_download_at_1080p_for_high_quality() {
        assert_eq!(downloaded_height(Quality::High), 1080);
    }

    #[test]
    #[ignore = "real YouTube: run scripts/run-tests.sh --only contract"]
    fn it_should_download_at_720p_for_mid_quality() {
        assert_eq!(downloaded_height(Quality::Mid), 720);
    }

    #[test]
    #[ignore = "real YouTube: run scripts/run-tests.sh --only contract"]
    fn it_should_download_at_480p_for_low_quality() {
        assert_eq!(downloaded_height(Quality::Low), 480);
    }

    #[test]
    #[ignore = "real YouTube: run scripts/run-tests.sh --only contract"]
    fn it_should_probe_the_height_a_quality_would_download() {
        let repository = YtDlpVideoDownloaderRepository::new(ytdlp_path());

        let probe = repository
            .probe_quality(&video_url(SHORT_VIDEO_ID), Quality::Mid)
            .unwrap();

        // Whether YouTube runs its SABR streaming experiment varies run to
        // run, so `degraded` is taken as reported; the height only holds
        // when it didn't.
        let expected_height = if probe.degraded {
            probe.height
        } else {
            Some(720)
        };
        assert_eq!(
            probe,
            QualityProbe {
                height: expected_height,
                published_at: DateTime::from_timestamp(1_550_518_860, 0),
                degraded: probe.degraded,
            }
        );
    }

    #[test]
    #[ignore = "real YouTube: run scripts/run-tests.sh --only contract"]
    fn it_should_fail_to_download_a_missing_video() {
        let staging_dir = unique_temp_dir("contract-download-missing");
        let repository = YtDlpVideoDownloaderRepository::new(ytdlp_path());

        let attempt = repository
            .download(&video_url(MISSING_VIDEO_ID), Quality::Low, &staging_dir)
            .unwrap();

        assert_eq!(
            attempt,
            DownloadAttempt::Failed {
                stderr: Some(format!(
                    "ERROR: [youtube] {MISSING_VIDEO_ID}: This video is unavailable"
                )),
                sabr_notice: None,
            }
        );
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[ignore = "real YouTube: run scripts/run-tests.sh --only contract"]
    fn it_should_diagnose_no_failure_for_a_playable_video() {
        let repository = YtDlpVideoDownloaderRepository::new(ytdlp_path());

        let reason = repository.diagnose(&video_url(SHORT_VIDEO_ID)).unwrap();

        assert_eq!(reason, None);
    }

    #[test]
    #[ignore = "real YouTube: run scripts/run-tests.sh --only contract"]
    fn it_should_diagnose_why_a_missing_video_failed() {
        let repository = YtDlpVideoDownloaderRepository::new(ytdlp_path());

        let reason = repository.diagnose(&video_url(MISSING_VIDEO_ID)).unwrap();

        assert_eq!(reason, Some("This video is unavailable".to_string()));
    }

    /// Downloads the short fixture at `quality` and returns its video height.
    /// The fixture offers h264 up to 1080p plus YouTube's muxed 360p format
    /// `18`, the stream that used to win every tier (issue #116).
    fn downloaded_height(quality: Quality) -> u32 {
        let staging_dir = unique_temp_dir("contract-download-quality");
        let repository = YtDlpVideoDownloaderRepository::new(ytdlp_path());

        let attempt = repository
            .download(&video_url(SHORT_VIDEO_ID), quality, &staging_dir)
            .unwrap();

        let DownloadAttempt::Succeeded(video) = attempt else {
            panic!("expected a successful download, got {attempt:?}");
        };
        let height = video_height(&video.media_path);
        std::fs::remove_dir_all(&staging_dir).unwrap();
        assert_eq!(
            video.height,
            Some(height),
            "reported height matches the file"
        );
        height
    }
}
