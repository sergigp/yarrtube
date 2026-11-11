use crate::domain::services::{VideoUpdater, VideoUpdaterApi};
use crate::domain::video::{ChaptersProbe, Video, VideoChapter, VideoStepOutcome};
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
    /// video's chapters from YouTube and records the check under the
    /// `late-video-data` policy; a failed fetch is logged and counted as a
    /// failed attempt, re-queued under the `video-retries` policy. Fails only
    /// when the video can't be stored.
    fn fetch(&self, video: &Video) -> anyhow::Result<VideoStepOutcome>;
}

impl ChapterFetcherApi for ChapterFetcher {
    fn fetch(&self, video: &Video) -> anyhow::Result<VideoStepOutcome> {
        if !video.is_chapters_fetchable() {
            return Ok(VideoStepOutcome::Settled);
        }
        match self.fetch_chapters(video) {
            Ok(probe) => self
                .record_check(video, Some(probe.chapters), probe.published_at)
                .map(|_| VideoStepOutcome::Settled),
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to fetch video chapters");
                self.record_failed_fetch(video, e.to_string())
            }
        }
    }
}

impl ChapterFetcher {
    /// Records a failed fetch, failed for `reason`: Retrying, run again when
    /// its next attempt is due, or Failed.
    fn record_failed_fetch(
        &self,
        video: &Video,
        reason: String,
    ) -> anyhow::Result<VideoStepOutcome> {
        let recorded = self.record_check(video, None, None)?;
        Ok(VideoStepOutcome::retry_at_or_settled(
            recorded.and_then(|video| video.chapters.state.next_attempt_at()),
            reason,
        ))
    }

    fn fetch_chapters(&self, video: &Video) -> anyhow::Result<ChaptersProbe> {
        self.chapter_repository
            .fetch_chapters(&video.youtube_id.to_url())
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
    ) -> anyhow::Result<Option<Video>> {
        let now = self.clock.now();
        self.video_updater.update(&video.id, |video| {
            let published_at = published_at.unwrap_or(video.created_at);
            video
                .is_chapters_fetchable()
                .then(|| video.record_chapters_check(chapters.clone(), published_at, now))
        })
    }
}
