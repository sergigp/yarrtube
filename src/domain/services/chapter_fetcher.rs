use crate::domain::services::{VideoUpdater, VideoUpdaterApi};
use crate::domain::video::{Video, VideoChapter, VideoStepOutcome};
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_chapter_repository::ChapterRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tracing::warn;

/// Best-effort chapters fetch for a video downloaded before its chapters
/// were recorded — see the `video-chapters` capability. Runs from the
/// `fetch_chapters` task and never returns an error to its caller;
/// `VideoReconciler` decides when to schedule it.
#[derive(Clone)]
pub struct ChapterFetcher {
    video_updater: VideoUpdater,
    chapter_repository: Arc<dyn ChapterRepository>,
    clock: Arc<dyn Clock>,
}

impl ChapterFetcher {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        chapter_repository: Arc<dyn ChapterRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_updater: VideoUpdater::new(video_repository),
            chapter_repository,
            clock,
        }
    }
}

pub trait ChapterFetcherApi: Send + Sync {
    /// No-ops unless `video.is_chapters_fetchable()`. Otherwise reads the
    /// video's chapters from YouTube and records the check (chapters,
    /// checks, settled) under the `late-video-data` policy; a failure is
    /// logged, counted as a failed attempt and swallowed.
    fn fetch(&self, video: &Video) -> anyhow::Result<VideoStepOutcome>;
}

impl ChapterFetcherApi for ChapterFetcher {
    fn fetch(&self, video: &Video) -> anyhow::Result<VideoStepOutcome> {
        if !video.is_chapters_fetchable() {
            return Ok(VideoStepOutcome::Settled);
        }
        if let Err(e) = self.fetch_and_record(video) {
            warn!(video_id = %video.id, error = %e, "failed to fetch video chapters");
            self.record_failed_fetch(video);
        }
        Ok(VideoStepOutcome::Settled)
    }
}

impl ChapterFetcher {
    /// Best effort, like the fetch itself: a failure to record the failed
    /// fetch is only logged.
    fn record_failed_fetch(&self, video: &Video) {
        if let Err(e) = self.record_check(video, None, None) {
            warn!(video_id = %video.id, error = %e, "failed to record a failed chapters fetch");
        }
    }

    fn fetch_and_record(&self, video: &Video) -> anyhow::Result<()> {
        let probe = self
            .chapter_repository
            .fetch_chapters(&video.youtube_id.to_url())?;
        self.record_check(video, Some(probe.chapters), probe.published_at)
    }

    /// Re-reads the video before writing: the fetch may have run while the
    /// video was deleted or had its chapters settled by a redownload, so the
    /// check is recorded onto the stored row, or skipped when it no longer
    /// wants one, rather than written from the stale copy read when the fetch
    /// started.
    fn record_check(
        &self,
        video: &Video,
        chapters: Option<Vec<VideoChapter>>,
        published_at: Option<DateTime<Utc>>,
    ) -> anyhow::Result<()> {
        let now = self.clock.now();
        self.video_updater
            .update(&video.id, |video| {
                let published_at = published_at.unwrap_or(video.created_at);
                video
                    .is_chapters_fetchable()
                    .then(|| video.record_chapters_check(chapters.clone(), published_at, now))
            })
            .map(|_| ())
    }
}
