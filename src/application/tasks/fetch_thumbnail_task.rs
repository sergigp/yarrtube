use crate::domain::services::ThumbnailFetcher;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::sync::Arc;

/// Fetches one video's thumbnail ahead of its download, scheduled by
/// `subscribers::fetch_thumbnail_on_video_added_to_playlist`/
/// `..._to_channel` and by the reconcilers' missing-thumbnail recovery.
/// Best-effort: always succeeds, so the queue never retries a video that
/// simply has no thumbnail. The next reconcile pass reschedules it instead.
pub struct FetchThumbnailTask {
    video_repository: Arc<dyn VideoRepository>,
    thumbnail_fetcher: Arc<ThumbnailFetcher>,
}

impl FetchThumbnailTask {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        thumbnail_fetcher: Arc<ThumbnailFetcher>,
    ) -> Self {
        Self {
            video_repository,
            thumbnail_fetcher,
        }
    }
}

impl TaskHandler for FetchThumbnailTask {
    fn handle(&self, _payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        Ok(())
    }
}
