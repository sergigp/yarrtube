use crate::domain::services::{ThumbnailFetcher, ThumbnailFetcherApi};
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::path::Path;
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
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let (video_id, output_dir) = Task::decode_fetch_thumbnail_payload(payload)?;
        let video_id = VideoRecordId::new(video_id)?;
        let video = self
            .video_repository
            .find(&video_id)?
            .ok_or_else(|| anyhow::anyhow!("video {video_id} not found"))?;
        self.thumbnail_fetcher.fetch(&video, Path::new(&output_dir));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::video::{Video, VideoId};
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::{
        FakeVideoDownloaderRepository, FetchedThumbnail,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::path::PathBuf;

    #[test]
    fn it_should_fetch_and_record_the_thumbnail() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default().with_thumbnail_result(Some(
                FetchedThumbnail {
                    folder: "My Video".to_string(),
                    filename: "My Video.jpg".to_string(),
                },
            )),
        );
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = FetchThumbnailTask::new(
            video_repository.clone(),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.shared_connection(),
                    Arc::new(FixedClock(later())),
                )),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.with_thumbnail("My Video/My Video.jpg", later())]
        );
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![thumbnail_call()]
        );
    }

    fn my_video() -> Video {
        Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp())
    }

    /// One call as `FakeVideoDownloaderRepository` records it, for the `yt1`
    /// video with no recorded folder.
    fn thumbnail_call() -> (String, String, String, PathBuf, Option<String>) {
        (
            "https://www.youtube.com/watch?v=yt1".to_string(),
            "My Video".to_string(),
            "yt1".to_string(),
            PathBuf::from("/videos/my-playlist"),
            None,
        )
    }

    fn payload_for(video_id: &str) -> String {
        Task::FetchThumbnail {
            video_id: video_id.to_string(),
            output_dir: "/videos/my-playlist".to_string(),
        }
        .payload()
        .to_string()
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn later() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_600, 0).unwrap()
    }

    fn run(task: &FetchThumbnailTask, payload: &str) -> Result<(), String> {
        task.handle(payload, false).map_err(|e| e.to_string())
    }
}
