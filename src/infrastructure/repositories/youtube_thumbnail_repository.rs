#[cfg(test)]
use crate::domain::video::FetchedThumbnail;
use crate::domain::video::ThumbnailFetch;
use crate::infrastructure::shared::ytdlp;
use std::path::{Path, PathBuf};

/// Fetches a video's thumbnail alone via `yt-dlp`, injected into
/// `ThumbnailFetcher`.
pub trait ThumbnailRepository: Send + Sync {
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
}

/// Invokes `yt-dlp` at `ytdlp_path`, the same configured path `update-ytdlp`
/// installs to.
pub struct YtDlpThumbnailRepository {
    ytdlp_path: PathBuf,
}

impl YtDlpThumbnailRepository {
    pub fn new(ytdlp_path: PathBuf) -> Self {
        Self { ytdlp_path }
    }
}

impl ThumbnailRepository for YtDlpThumbnailRepository {
    fn fetch_thumbnail(
        &self,
        video_url: &str,
        staging_dir: &Path,
    ) -> anyhow::Result<ThumbnailFetch> {
        ytdlp::fetch_thumbnail(&self.ytdlp_path, video_url, staging_dir)
    }
}

/// Unconfigured, `fetch_thumbnail` mirrors a clean "no thumbnail available"
/// outcome.
#[cfg(test)]
#[derive(Default)]
pub struct FakeThumbnailRepository {
    #[allow(clippy::type_complexity)]
    pub(crate) result: std::sync::Mutex<Option<anyhow::Result<ThumbnailFetch>>>,
    /// The (url, staging dir) pairs `fetch_thumbnail` was called with.
    pub(crate) calls: std::sync::Mutex<Vec<(String, PathBuf)>>,
    /// Runs while `fetch_thumbnail` is in flight, e.g. to delete the video
    /// mid-fetch.
    #[allow(clippy::type_complexity)]
    pub(crate) on_fetch: Option<Box<dyn Fn() + Send + Sync>>,
}

#[cfg(test)]
impl FakeThumbnailRepository {
    /// Fetches `fetched` (its path relative to the staging dir), or, for
    /// `None`, finds no thumbnail available.
    pub fn with_result(fetched: Option<FetchedThumbnail>) -> Self {
        Self {
            result: std::sync::Mutex::new(Some(Ok(match fetched {
                Some(fetched) => ThumbnailFetch::Fetched(fetched),
                None => ThumbnailFetch::Unavailable { reason: None },
            }))),
            ..Self::default()
        }
    }

    /// Makes `fetch_thumbnail` return `Err`.
    pub fn failing() -> Self {
        Self {
            result: std::sync::Mutex::new(Some(Err(anyhow::anyhow!("fake thumbnail fetch error")))),
            ..Self::default()
        }
    }

    /// Runs `hook` while `fetch_thumbnail` is in flight.
    pub fn with_on_fetch(self, hook: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            on_fetch: Some(Box::new(hook)),
            ..self
        }
    }
}

#[cfg(test)]
impl ThumbnailRepository for FakeThumbnailRepository {
    /// A fetched thumbnail's path is relative to the staging dir it was
    /// asked to fetch into, as `yt-dlp` would report it.
    fn fetch_thumbnail(
        &self,
        video_url: &str,
        staging_dir: &Path,
    ) -> anyhow::Result<ThumbnailFetch> {
        self.calls
            .lock()
            .unwrap()
            .push((video_url.to_string(), staging_dir.to_path_buf()));
        if let Some(hook) = &self.on_fetch {
            hook();
        }
        match self.result.lock().unwrap().take() {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::ytdlp::test_support;

    #[test]
    #[cfg(unix)]
    fn it_should_map_a_successful_thumbnail_fetch_to_the_printed_filename() {
        let fake = test_support::FakeYtDlp::with_stdout(
            r#"{"thumbnails.-1.filepath": "My Video.jpg"}
"#,
        );
        let output_dir = test_support::unique_temp_dir("video-downloader-repository-thumbnail");

        let result = YtDlpThumbnailRepository::new(fake.path.clone())
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

/// Against the real `yt-dlp` and YouTube: run with
/// `scripts/run-tests.sh --contract` (see `shared::youtube_contract`).
#[cfg(test)]
mod youtube_contract_tests {
    use super::*;
    use crate::infrastructure::shared::youtube_contract::{SHORT_VIDEO_ID, video_url, ytdlp_path};
    use crate::infrastructure::shared::ytdlp::test_support::unique_temp_dir;
    use chrono::DateTime;

    #[test]
    #[ignore = "real YouTube: run scripts/run-tests.sh --contract"]
    fn it_should_fetch_only_the_thumbnail() {
        let staging_dir = unique_temp_dir("contract-thumbnail");
        let repository = YtDlpThumbnailRepository::new(ytdlp_path());

        let fetch = repository
            .fetch_thumbnail(&video_url(SHORT_VIDEO_ID), &staging_dir)
            .unwrap();

        let thumbnail = staging_dir.join(format!("{SHORT_VIDEO_ID}.jpg"));
        assert_eq!(
            fetch,
            ThumbnailFetch::Fetched(FetchedThumbnail {
                path: thumbnail.clone(),
                published_at: DateTime::from_timestamp(1_550_518_860, 0),
            })
        );
        assert_eq!(staged_files(&staging_dir), vec![thumbnail]);
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    fn staged_files(staging_dir: &Path) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(staging_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        files.sort();
        files
    }
}
