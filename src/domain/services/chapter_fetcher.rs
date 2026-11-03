use crate::domain::video::{Video, VideoChapter};
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::VideoDownloaderRepository;
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
    video_repository: Arc<dyn VideoRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    clock: Arc<dyn Clock>,
}

impl ChapterFetcher {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_repository,
            video_downloader_repository,
            clock,
        }
    }
}

pub trait ChapterFetcherApi: Send + Sync {
    /// No-ops unless `video.is_chapters_fetchable()`. Otherwise reads the
    /// video's chapters from YouTube and records the check (chapters,
    /// checks, settled) under the `late-video-data` policy; a failure is
    /// logged, counted as a failed attempt and swallowed.
    fn fetch(&self, video: &Video);
}

impl ChapterFetcherApi for ChapterFetcher {
    fn fetch(&self, video: &Video) {
        if !video.is_chapters_fetchable() {
            return;
        }
        if let Err(e) = self.fetch_and_record(video) {
            warn!(video_id = %video.id, error = %e, "failed to fetch video chapters");
            self.record_failed_fetch(video);
        }
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
            .video_downloader_repository
            .fetch_chapters(&video.youtube_id.to_url())?;
        self.record_check(video, Some(probe.chapters), probe.published_at)
    }

    fn record_check(
        &self,
        video: &Video,
        chapters: Option<Vec<VideoChapter>>,
        published_at: Option<DateTime<Utc>>,
    ) -> anyhow::Result<()> {
        self.video_repository
            .update_chapters(&video.clone().record_chapters_check(
                chapters,
                published_at.unwrap_or(video.created_at),
                self.clock.now(),
            ))
    }
}
