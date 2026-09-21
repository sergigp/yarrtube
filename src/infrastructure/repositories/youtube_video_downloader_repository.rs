use crate::domain::shared::Quality;
use crate::infrastructure::shared::ytdlp;
pub use crate::infrastructure::shared::ytdlp::{DownloadedVideo, FetchedThumbnail};
use std::path::{Path, PathBuf};

/// Downloads a single video (or just its thumbnail) via `yt-dlp`, injected
/// into `VideoDownloader`/`ThumbnailFetcher` for the event-driven download
/// path.
pub trait VideoDownloaderRepository: Send + Sync {
    /// Returns `Ok(Some(DownloadedVideo))` with the exact filename `yt-dlp`
    /// saved (and its duration, when known) on a successful download,
    /// `Ok(None)` for a clean `yt-dlp` failure (non-zero exit). Returns
    /// `Err` only for a systemic problem (e.g. `yt-dlp` missing from
    /// `PATH`, or an unparseable `--print` output). `existing_folder`, when
    /// `Some`, is reused verbatim as the video's per-video output folder
    /// instead of resolving a fresh one — see `video-download`'s "Video's
    /// thumbnail was already fetched ahead of its download" scenario.
    fn download(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        quality: Quality,
        output_dir: &Path,
        existing_folder: Option<&str>,
    ) -> anyhow::Result<Option<DownloadedVideo>>;

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
    ) -> anyhow::Result<Option<DownloadedVideo>> {
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
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeVideoDownloaderRepository {
    pub(crate) result: std::sync::Mutex<Option<DownloadedVideo>>,
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
            ..Default::default()
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
            ..Default::default()
        }
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

    pub fn thumbnail_calls_count(&self) -> usize {
        self.thumbnail_calls.lock().unwrap().len()
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
    ) -> anyhow::Result<Option<DownloadedVideo>> {
        self.calls.lock().unwrap().push((
            video_url.to_string(),
            desired_filename.to_string(),
            video_id.to_string(),
            quality,
            output_dir.to_path_buf(),
            existing_folder.map(str::to_string),
        ));
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
                None,
            )
            .unwrap();

        assert_eq!(result, None);
        std::fs::remove_dir_all(&output_dir).unwrap();
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
