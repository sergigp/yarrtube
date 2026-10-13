use crate::domain::services::{ThumbnailFetcher, ThumbnailFetcherApi};
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::path::Path;
use std::sync::Arc;
use tracing::debug;

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
        let Some(video) = self.video_repository.find(&video_id)? else {
            debug!(video_id = %video_id, "video no longer exists, skipping thumbnail fetch");
            return Ok(());
        };
        self.thumbnail_fetcher.fetch(&video, Path::new(&output_dir));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::ChannelHandle;
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::video::{EpisodeNumber, Video, VideoId};
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, SqliteChannelVideoRepository,
    };
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
    fn it_should_place_a_prefetched_thumbnail_as_the_episode_image() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default().with_thumbnail_result(Some(
                FetchedThumbnail {
                    path: PathBuf::from("yt1.jpg"),
                    published_at: Some(published_at()),
                },
            )),
        );
        let video = my_video();
        video_repository.save(&video).unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = FetchThumbnailTask::new(
            video_repository.clone(),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(later())),
                )),
                Arc::new(FixedClock(later())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .clone()
                    .with_episode_number(EpisodeNumber::from_columns(2026, 31501).unwrap(), later())
                    .with_thumbnail("Season 2026/S2026E031501 - My Video.jpg", later())
            ]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                PathBuf::from(format!("/videos/.downloads/{}/yt1.jpg", video.id.as_str())),
                PathBuf::from("/videos/my-playlist/Season 2026/S2026E031501 - My Video.jpg"),
            )]
        );
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from(format!(
                "/videos/.downloads/{}",
                video.id.as_str()
            ))]
        );
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![thumbnail_call(&video)]
        );
    }

    #[test]
    fn it_should_skip_if_video_already_has_a_thumbnail() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_downloader_repository = Arc::new(FakeVideoDownloaderRepository::default());
        let video = my_video().with_thumbnail("My Video/My Video.jpg", fixed_timestamp());
        video_repository.save(&video).unwrap();
        let task = FetchThumbnailTask::new(
            video_repository.clone(),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(later())),
                )),
                Arc::new(FixedClock(later())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![]
        );
    }

    #[test]
    fn it_should_not_fetch_a_thumbnail_for_an_excluded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default().with_thumbnail_result(Some(
                FetchedThumbnail {
                    path: std::path::PathBuf::from("My Video.jpg"),
                    published_at: None,
                },
            )),
        );
        let video = my_video().mark_excluded(fixed_timestamp());
        video_repository.save(&video).unwrap();
        let task = FetchThumbnailTask::new(
            video_repository.clone(),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(later())),
                )),
                Arc::new(FixedClock(later())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![]
        );
    }

    #[test]
    fn it_should_not_fetch_a_thumbnail_for_an_errored_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default().with_thumbnail_result(Some(
                FetchedThumbnail {
                    path: std::path::PathBuf::from("My Video.jpg"),
                    published_at: None,
                },
            )),
        );
        let video = my_video().mark_errored(fixed_timestamp());
        video_repository.save(&video).unwrap();
        let task = FetchThumbnailTask::new(
            video_repository.clone(),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(later())),
                )),
                Arc::new(FixedClock(later())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![]
        );
    }

    #[test]
    fn it_should_skip_if_video_no_longer_exists() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_downloader_repository = Arc::new(FakeVideoDownloaderRepository::default());
        let task = FetchThumbnailTask::new(
            video_repository.clone(),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(later())),
                )),
                Arc::new(FixedClock(later())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for("missing"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![]
        );
    }

    #[test]
    fn it_should_leave_the_video_untouched_when_the_thumbnail_fetch_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository =
            Arc::new(FakeVideoDownloaderRepository::default().with_thumbnail_error());
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = FetchThumbnailTask::new(
            video_repository.clone(),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(later())),
                )),
                Arc::new(FixedClock(later())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from(format!(
                "/videos/.downloads/{}",
                video.id.as_str()
            ))]
        );
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![thumbnail_call(&video)]
        );
    }

    fn my_channel_id() -> ChannelHandle {
        ChannelHandle::new("@somechannel").unwrap()
    }

    fn my_video() -> Video {
        Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp())
    }

    /// One call as `FakeVideoDownloaderRepository` records it, for the `yt1`
    /// video, staged under its record id.
    fn thumbnail_call(video: &Video) -> (String, PathBuf) {
        (
            "https://www.youtube.com/watch?v=yt1".to_string(),
            PathBuf::from(format!("/videos/.downloads/{}", video.id.as_str())),
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

    /// The publish time `yt-dlp` reports with the thumbnail.
    fn published_at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-03-15T10:00:00Z")
            .unwrap()
            .to_utc()
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
