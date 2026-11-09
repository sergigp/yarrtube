use crate::domain::video::ChaptersProbe;
#[cfg(test)]
use crate::domain::video::VideoChapter;
use crate::infrastructure::shared::ytdlp;
use std::path::PathBuf;

/// Reads a video's chapters via `yt-dlp`, injected into `ChapterFetcher`.
pub trait ChapterRepository: Send + Sync {
    /// The video's chapters (`vec![]` when it has none) and publish time,
    /// downloading nothing. `Err` on any failure, clean or systemic.
    fn fetch_chapters(&self, video_url: &str) -> anyhow::Result<ChaptersProbe>;
}

/// Invokes `yt-dlp` at `ytdlp_path`, the same configured path `update-ytdlp`
/// installs to.
pub struct YtDlpChapterRepository {
    ytdlp_path: PathBuf,
}

impl YtDlpChapterRepository {
    pub fn new(ytdlp_path: PathBuf) -> Self {
        Self { ytdlp_path }
    }
}

impl ChapterRepository for YtDlpChapterRepository {
    fn fetch_chapters(&self, video_url: &str) -> anyhow::Result<ChaptersProbe> {
        ytdlp::fetch_chapters(&self.ytdlp_path, video_url)
    }
}

/// Unconfigured, `fetch_chapters` fails.
#[cfg(test)]
#[derive(Default)]
pub struct FakeChapterRepository {
    pub(crate) result: std::sync::Mutex<Option<anyhow::Result<ChaptersProbe>>>,
    /// The video URLs `fetch_chapters` was called with.
    pub(crate) calls: std::sync::Mutex<Vec<String>>,
    /// Runs while `fetch_chapters` is in flight, e.g. to record watch
    /// progress mid-fetch.
    #[allow(clippy::type_complexity)]
    pub(crate) on_fetch: Option<Box<dyn Fn() + Send + Sync>>,
}

#[cfg(test)]
impl FakeChapterRepository {
    /// Returns `result` as the chapters, with no publish time.
    pub fn with_chapters(result: anyhow::Result<Vec<VideoChapter>>) -> Self {
        Self::with_probe(result.map(|chapters| ChaptersProbe {
            chapters,
            published_at: None,
        }))
    }

    pub fn with_probe(result: anyhow::Result<ChaptersProbe>) -> Self {
        Self {
            result: std::sync::Mutex::new(Some(result)),
            ..Self::default()
        }
    }

    /// Runs `hook` while `fetch_chapters` is in flight.
    pub fn with_on_fetch(self, hook: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            on_fetch: Some(Box::new(hook)),
            ..self
        }
    }
}

#[cfg(test)]
impl ChapterRepository for FakeChapterRepository {
    fn fetch_chapters(&self, video_url: &str) -> anyhow::Result<ChaptersProbe> {
        self.calls.lock().unwrap().push(video_url.to_string());
        if let Some(hook) = &self.on_fetch {
            hook();
        }
        self.result
            .lock()
            .unwrap()
            .take()
            .unwrap_or_else(|| Err(anyhow::anyhow!("no fake chapters configured")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::ytdlp::test_support;

    #[test]
    #[cfg(unix)]
    fn it_should_map_the_printed_chapters_of_a_yt_dlp_process() {
        let fake = test_support::FakeYtDlp::with_stdout(
            r#"{"chapters": [{"start_time": 0.0, "title": "Intro", "end_time": 61.7}, {"start_time": 61.7, "title": "Main part", "end_time": 300.0}]}
"#,
        );

        let result = YtDlpChapterRepository::new(fake.path.clone())
            .fetch_chapters("https://example.com/video")
            .unwrap()
            .chapters;

        assert_eq!(
            result,
            vec![
                VideoChapter {
                    start_seconds: 0,
                    end_seconds: 61,
                    title: "Intro".to_string(),
                },
                VideoChapter {
                    start_seconds: 61,
                    end_seconds: 300,
                    title: "Main part".to_string(),
                },
            ]
        );
    }
}

/// Against the real `yt-dlp` and YouTube: run with
/// `scripts/run-tests.sh --only contract` (see `shared::youtube_contract`).
#[cfg(test)]
mod youtube_contract_tests {
    use super::*;
    use crate::infrastructure::shared::youtube_contract::{
        CHAPTERED_VIDEO_ID, flamenco_chapters, video_url, ytdlp_path,
    };

    #[test]
    #[ignore = "real YouTube: run scripts/run-tests.sh --only contract"]
    fn it_should_fetch_the_chapters_of_a_real_video() {
        let repository = YtDlpChapterRepository::new(ytdlp_path());

        let chapters = repository
            .fetch_chapters(&video_url(CHAPTERED_VIDEO_ID))
            .unwrap()
            .chapters;

        assert_eq!(chapters, flamenco_chapters());
    }
}
