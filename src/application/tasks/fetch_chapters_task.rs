use crate::domain::services::{ChapterFetcher, ChapterFetcherApi};
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::sync::Arc;

/// Fetches one downloaded video's chapters, scheduled by the video sync
/// (`SyncVideoTask` via `VideoReconciler`) while its chapters are unchecked.
/// Best-effort: always succeeds, so the queue never retries it; the next
/// reconcile pass reschedules a fetch that failed.
pub struct FetchChaptersTask {
    video_repository: Arc<dyn VideoRepository>,
    chapter_fetcher: Arc<ChapterFetcher>,
}

impl FetchChaptersTask {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        chapter_fetcher: Arc<ChapterFetcher>,
    ) -> Self {
        Self {
            video_repository,
            chapter_fetcher,
        }
    }
}

impl TaskHandler for FetchChaptersTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let video_id = VideoRecordId::new(Task::decode_fetch_chapters_payload(payload)?)?;
        if let Some(video) = self.video_repository.find(&video_id)? {
            self.chapter_fetcher.fetch(&video);
        }
        Ok(())
    }
}
