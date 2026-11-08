use crate::domain::video_transcript::{CaptionProbe, CaptionTrack, FetchedSubtitles};
use crate::infrastructure::shared::ytdlp;
use std::path::{Path, PathBuf};

/// Reads a video's caption tracks and downloads one of them via `yt-dlp`,
/// injected into `SubtitleFetcher`.
pub trait CaptionRepository: Send + Sync {
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
/// installs to.
pub struct YtDlpCaptionRepository {
    ytdlp_path: PathBuf,
}

impl YtDlpCaptionRepository {
    pub fn new(ytdlp_path: PathBuf) -> Self {
        Self { ytdlp_path }
    }
}

impl CaptionRepository for YtDlpCaptionRepository {
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

/// Unconfigured, both `probe_captions` and `fetch_subtitles` fail.
#[cfg(test)]
#[derive(Default)]
pub struct FakeCaptionRepository {
    pub(crate) probe: std::sync::Mutex<Option<anyhow::Result<CaptionProbe>>>,
    pub(crate) subtitles: std::sync::Mutex<Option<anyhow::Result<FetchedSubtitles>>>,
    /// How many times YouTube was contacted for captions, probe or download.
    pub(crate) calls: std::sync::Mutex<usize>,
    /// Runs while `fetch_subtitles` is in flight, e.g. to delete the video
    /// mid-fetch.
    #[allow(clippy::type_complexity)]
    pub(crate) on_fetch_subtitles: Option<Box<dyn Fn() + Send + Sync>>,
}

#[cfg(test)]
impl FakeCaptionRepository {
    /// Configures what `probe_captions` returns.
    pub fn with_probe(probe: anyhow::Result<CaptionProbe>) -> Self {
        Self {
            probe: std::sync::Mutex::new(Some(probe)),
            ..Self::default()
        }
    }

    /// Configures what `fetch_subtitles` returns, its path relative to the
    /// staging dir.
    pub fn with_subtitles(self, result: anyhow::Result<FetchedSubtitles>) -> Self {
        Self {
            subtitles: std::sync::Mutex::new(Some(result)),
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
}

#[cfg(test)]
impl CaptionRepository for FakeCaptionRepository {
    fn probe_captions(
        &self,
        _video_url: &str,
        _staging_dir: &Path,
    ) -> anyhow::Result<CaptionProbe> {
        *self.calls.lock().unwrap() += 1;
        self.probe
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
        *self.calls.lock().unwrap() += 1;
        if let Some(hook) = &self.on_fetch_subtitles {
            hook();
        }
        self.subtitles
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

/// Against the real `yt-dlp` and YouTube: run with
/// `scripts/run-tests.sh --only contract` (see `shared::youtube_contract`).
#[cfg(test)]
mod youtube_contract_tests {
    use super::*;
    use crate::domain::video_transcript::{CaptionLanguage, CaptionSource, CaptionTracks};
    use crate::infrastructure::shared::youtube_contract::{
        CAPTIONED_VIDEO_ID, video_url, ytdlp_path,
    };
    use crate::infrastructure::shared::ytdlp::test_support::unique_temp_dir;
    use chrono::DateTime;

    #[test]
    #[ignore = "real YouTube: run scripts/run-tests.sh --only contract"]
    fn it_should_probe_the_caption_tracks() {
        let staging_dir = unique_temp_dir("contract-captions");
        let repository = YtDlpCaptionRepository::new(ytdlp_path());

        let probe = repository
            .probe_captions(&video_url(CAPTIONED_VIDEO_ID), &staging_dir)
            .unwrap();

        // YouTube's auto-translated captions (over a hundred languages) and
        // the warnings it prints alongside vary run to run, so they're taken
        // as reported; the English auto-captions must be among them.
        assert!(probe.tracks.automatic.contains(&"en".to_string()));
        assert_eq!(
            probe,
            CaptionProbe {
                tracks: CaptionTracks {
                    original_language: Some("en".to_string()),
                    manual: vec!["en".to_string(), "fr".to_string()],
                    automatic: probe.tracks.automatic.clone(),
                },
                published_at: DateTime::from_timestamp(1_212_060_266, 0),
                info_json: staging_dir.join("info.json"),
                warnings: probe.warnings.clone(),
            }
        );
        assert!(probe.info_json.is_file());
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }

    #[test]
    #[ignore = "real YouTube: run scripts/run-tests.sh --only contract"]
    fn it_should_fetch_the_uploaded_subtitles_as_srt() {
        let staging_dir = unique_temp_dir("contract-subtitles");
        let repository = YtDlpCaptionRepository::new(ytdlp_path());
        let probe = repository
            .probe_captions(&video_url(CAPTIONED_VIDEO_ID), &staging_dir)
            .unwrap();
        let english = CaptionTrack {
            key: "en".to_string(),
            language: CaptionLanguage::new("en").unwrap(),
            source: CaptionSource::Manual,
        };

        let subtitles = repository
            .fetch_subtitles(&probe, &english, &staging_dir)
            .unwrap();

        assert_eq!(
            subtitles,
            FetchedSubtitles {
                path: staging_dir.join(format!("{CAPTIONED_VIDEO_ID}.en.srt")),
                srt: subtitles.srt.clone(),
            }
        );
        // SubRip: numbered cues, each with a `start --> end` timing line.
        let mut lines = subtitles.srt.lines();
        assert_eq!(lines.next(), Some("1"));
        assert!(lines.next().is_some_and(|timing| timing.contains(" --> ")));
        std::fs::remove_dir_all(&staging_dir).unwrap();
    }
}
