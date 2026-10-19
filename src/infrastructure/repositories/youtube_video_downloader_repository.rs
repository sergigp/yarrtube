use crate::domain::shared::Quality;
use crate::domain::video_transcript::CaptionTrack;
use crate::infrastructure::shared::ytdlp;
pub use crate::infrastructure::shared::ytdlp::{
    CaptionProbe, DownloadAttempt, DownloadedVideo, FetchedSubtitles, FetchedThumbnail,
    ThumbnailFetch,
};
use std::path::{Path, PathBuf};

/// Downloads a single video (or just its thumbnail) via `yt-dlp`, injected
/// into `VideoDownloader`/`ThumbnailFetcher` for the event-driven download
/// path.
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

    /// Fetches only the video's thumbnail into `staging_dir`. Returns
    /// `Ok(ThumbnailFetch::Fetched(..))` when a thumbnail was fetched,
    /// `Ok(ThumbnailFetch::Unavailable { reason })` when none could be
    /// obtained, carrying `yt-dlp`'s reported error text when it reported one
    /// (not an error — see `video-thumbnails`). Returns `Err` only for a
    /// systemic problem.
    fn fetch_thumbnail(
        &self,
        video_url: &str,
        staging_dir: &Path,
    ) -> anyhow::Result<ThumbnailFetch>;

    /// Runs a simulate-only probe (forcing the alternate player clients) to
    /// reveal the *precise* reason a download failed, since `yt-dlp`'s default
    /// clients collapse many permanent blocks into a bare "Video unavailable".
    /// Returns `Ok(Some(reason))` with the reason `yt-dlp` printed,
    /// `Ok(None)` when none can be determined (clean exit, or only a bare
    /// "Video unavailable"), and `Err` only for a systemic problem. The probe
    /// never downloads — it only diagnoses.
    fn diagnose(&self, video_url: &str) -> anyhow::Result<Option<String>>;

    /// Reads the video's caption tracks, original language and publish time
    /// into `staging_dir`, downloading nothing. `Err` on any failure, clean
    /// or systemic.
    fn probe_captions(&self, video_url: &str, staging_dir: &Path) -> anyhow::Result<CaptionProbe>;

    /// Downloads only `track` of the probed video into `staging_dir`, as
    /// SubRip. `Err` on any failure, including no `.srt` written.
    fn fetch_subtitles(
        &self,
        probe: &CaptionProbe,
        track: &CaptionTrack,
        staging_dir: &Path,
    ) -> anyhow::Result<FetchedSubtitles>;
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

    fn fetch_thumbnail(
        &self,
        video_url: &str,
        staging_dir: &Path,
    ) -> anyhow::Result<ThumbnailFetch> {
        ytdlp::fetch_thumbnail(&self.ytdlp_path, video_url, staging_dir)
    }

    fn diagnose(&self, video_url: &str) -> anyhow::Result<Option<String>> {
        ytdlp::diagnose(&self.ytdlp_path, video_url)
    }

    fn probe_captions(&self, video_url: &str, staging_dir: &Path) -> anyhow::Result<CaptionProbe> {
        ytdlp::probe_captions(&self.ytdlp_path, video_url, staging_dir)
    }

    fn fetch_subtitles(
        &self,
        probe: &CaptionProbe,
        track: &CaptionTrack,
        staging_dir: &Path,
    ) -> anyhow::Result<FetchedSubtitles> {
        ytdlp::fetch_subtitles(&self.ytdlp_path, probe, track, staging_dir)
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
    #[allow(clippy::type_complexity)]
    pub(crate) thumbnail_result: std::sync::Mutex<Option<anyhow::Result<ThumbnailFetch>>>,
    #[allow(clippy::type_complexity)]
    pub(crate) thumbnail_calls: std::sync::Mutex<Vec<(String, std::path::PathBuf)>>,
    /// Runs while `download` is in flight, e.g. to delete the video
    /// mid-download.
    #[allow(clippy::type_complexity)]
    pub(crate) on_download: Option<Box<dyn Fn() + Send + Sync>>,
    /// Runs while `fetch_subtitles` is in flight, e.g. to delete the video
    /// mid-fetch.
    #[allow(clippy::type_complexity)]
    pub(crate) on_fetch_subtitles: Option<Box<dyn Fn() + Send + Sync>>,
    pub(crate) caption_probe: std::sync::Mutex<Option<anyhow::Result<CaptionProbe>>>,
    pub(crate) subtitles_result: std::sync::Mutex<Option<anyhow::Result<FetchedSubtitles>>>,
    /// How many times the downloader was contacted for captions, probe or
    /// download.
    pub(crate) caption_calls: std::sync::Mutex<usize>,
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
                media_path: PathBuf::from("fake-output.mp4"),
                thumbnail_path: None,
                duration_seconds: None,
                published_at: None,
                sabr_notice: None,
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
            media_path: PathBuf::from("fake-output.mp4"),
            thumbnail_path: None,
            duration_seconds: None,
            published_at: None,
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
            thumbnail_result: std::sync::Mutex::new(Some(Ok(match result {
                Some(fetched) => ThumbnailFetch::Fetched(fetched),
                None => ThumbnailFetch::Unavailable { reason: None },
            }))),
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

    /// Configures what `probe_captions` returns.
    pub fn with_caption_probe(self, probe: anyhow::Result<CaptionProbe>) -> Self {
        Self {
            caption_probe: std::sync::Mutex::new(Some(probe)),
            ..self
        }
    }

    /// Configures what `fetch_subtitles` returns, its path relative to the
    /// staging dir.
    pub fn with_subtitles_result(self, result: anyhow::Result<FetchedSubtitles>) -> Self {
        Self {
            subtitles_result: std::sync::Mutex::new(Some(result)),
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

    /// Runs `hook` while `fetch_subtitles` is in flight.
    pub fn with_on_fetch_subtitles(self, hook: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            on_fetch_subtitles: Some(Box::new(hook)),
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
            on_fetch_subtitles: None,
            caption_probe: Default::default(),
            subtitles_result: Default::default(),
            caption_calls: Default::default(),
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

    fn fetch_thumbnail(
        &self,
        video_url: &str,
        staging_dir: &Path,
    ) -> anyhow::Result<ThumbnailFetch> {
        self.thumbnail_calls
            .lock()
            .unwrap()
            .push((video_url.to_string(), staging_dir.to_path_buf()));
        match self.thumbnail_result.lock().unwrap().take() {
            Some(Ok(ThumbnailFetch::Fetched(fetched))) => {
                Ok(ThumbnailFetch::Fetched(FetchedThumbnail {
                    path: staging_dir.join(&fetched.path),
                    ..fetched
                }))
            }
            Some(result) => result,
            None => Ok(ThumbnailFetch::Unavailable { reason: None }),
        }
    }

    fn diagnose(&self, _video_url: &str) -> anyhow::Result<Option<String>> {
        match self.diagnose_result.lock().unwrap().clone() {
            DiagnoseOutcome::Reason(reason) => Ok(reason),
            DiagnoseOutcome::Error => Err(anyhow::anyhow!("fake diagnose error")),
        }
    }

    fn probe_captions(
        &self,
        _video_url: &str,
        _staging_dir: &Path,
    ) -> anyhow::Result<CaptionProbe> {
        *self.caption_calls.lock().unwrap() += 1;
        self.caption_probe
            .lock()
            .unwrap()
            .take()
            .unwrap_or_else(|| Err(anyhow::anyhow!("no fake caption probe configured")))
    }

    fn fetch_subtitles(
        &self,
        _probe: &CaptionProbe,
        _track: &CaptionTrack,
        staging_dir: &Path,
    ) -> anyhow::Result<FetchedSubtitles> {
        *self.caption_calls.lock().unwrap() += 1;
        if let Some(hook) = &self.on_fetch_subtitles {
            hook();
        }
        self.subtitles_result
            .lock()
            .unwrap()
            .take()
            .unwrap_or_else(|| Err(anyhow::anyhow!("no fake subtitles configured")))
            .map(|fetched| FetchedSubtitles {
                path: staging_dir.join(&fetched.path),
                ..fetched
            })
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
                media_path: output_dir.join(test_support::DEFAULT_PRINTED_FILENAME),
                thumbnail_path: None,
                duration_seconds: None,
                published_at: None,
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

    #[test]
    #[cfg(unix)]
    fn it_should_map_a_successful_thumbnail_fetch_to_the_printed_filename() {
        let fake = test_support::FakeYtDlp::with_stdout("My Video.jpg\n");
        let output_dir = test_support::unique_temp_dir("video-downloader-repository-thumbnail");

        let result = YtDlpVideoDownloaderRepository::new(fake.path.clone())
            .fetch_thumbnail("https://example.com/video", &output_dir)
            .unwrap();

        assert_eq!(
            result,
            ThumbnailFetch::Fetched(FetchedThumbnail {
                path: output_dir.join("My Video.jpg"),
                published_at: None,
            })
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }
}
