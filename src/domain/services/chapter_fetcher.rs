use crate::domain::video::Video;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::VideoDownloaderRepository;
use crate::infrastructure::shared::system_clock::Clock;
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
    /// video's chapters from YouTube and records them, writing only the
    /// chapters; any failure is logged and swallowed, leaving the chapters
    /// unchecked so a later video sync tries again.
    fn fetch(&self, video: &Video);
}

impl ChapterFetcherApi for ChapterFetcher {
    fn fetch(&self, video: &Video) {
        if !video.is_chapters_fetchable() {
            return;
        }
        if let Err(e) = self.fetch_and_record(video) {
            warn!(video_id = %video.id, error = %e, "failed to fetch video chapters");
        }
    }
}

impl ChapterFetcher {
    fn fetch_and_record(&self, video: &Video) -> anyhow::Result<()> {
        let chapters = self
            .video_downloader_repository
            .fetch_chapters(&video.youtube_id.to_url())?;
        self.video_repository
            .update_chapters(&video.clone().with_chapters(chapters, self.clock.now()))
    }
}
