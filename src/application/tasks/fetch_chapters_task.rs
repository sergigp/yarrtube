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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::shared::Quality;
    use crate::domain::video::{PlaybackPosition, Video, VideoChapter, VideoId};
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    #[test]
    fn it_should_record_the_fetched_chapters() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default().with_chapters_result(Ok(intro_chapters())),
        );
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                chapters: Some(intro_chapters()),
                updated_at: later(),
                ..video
            }]
        );
        assert_eq!(
            *video_downloader_repository.chapters_calls.lock().unwrap(),
            vec![YT1_URL.to_string()]
        );
    }

    #[test]
    fn it_should_record_no_chapters_if_the_video_has_none() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_downloader_repository =
            Arc::new(FakeVideoDownloaderRepository::default().with_chapters_result(Ok(vec![])));
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                chapters: Some(vec![]),
                updated_at: later(),
                ..video
            }]
        );
    }

    #[test]
    fn it_should_leave_chapters_unchecked_if_the_fetch_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_chapters_result(Err(anyhow::anyhow!("Video unavailable"))),
        );
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(
            *video_downloader_repository.chapters_calls.lock().unwrap(),
            vec![YT1_URL.to_string()]
        );
    }

    #[test]
    fn it_should_skip_if_chapters_already_checked() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default().with_chapters_result(Ok(intro_chapters())),
        );
        let video = downloaded_video().with_chapters(vec![], fixed_timestamp());
        video_repository.save(&video).unwrap();
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(
            *video_downloader_repository.chapters_calls.lock().unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn it_should_skip_if_the_video_is_gone() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default().with_chapters_result(Ok(intro_chapters())),
        );
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &payload_for(&downloaded_video()));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            *video_downloader_repository.chapters_calls.lock().unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn it_should_keep_a_playback_position_recorded_during_the_fetch() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let played = video.clone().update_watch_state(
            PlaybackPosition::new(120).unwrap(),
            None,
            false,
            fixed_timestamp(),
        );
        let video_downloader_repository = Arc::new({
            let video_repository = video_repository.clone();
            let played = played.clone();
            FakeVideoDownloaderRepository::default()
                .with_chapters_result(Ok(intro_chapters()))
                .with_on_fetch_chapters(move || video_repository.update(&played).unwrap())
        });
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                video_downloader_repository,
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                chapters: Some(intro_chapters()),
                updated_at: later(),
                ..played
            }]
        );
    }

    const YT1_URL: &str = "https://www.youtube.com/watch?v=yt1";

    /// `yt1`, downloaded with its chapters never checked.
    fn downloaded_video() -> Video {
        Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp())
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "Season 2026/S2026E031501 - My Video.mp4",
                None,
                Some(600),
                fixed_timestamp(),
            )
    }

    fn intro_chapters() -> Vec<VideoChapter> {
        vec![
            VideoChapter {
                start_seconds: 0,
                end_seconds: 60,
                title: "Intro".to_string(),
            },
            VideoChapter {
                start_seconds: 60,
                end_seconds: 600,
                title: "Main".to_string(),
            },
        ]
    }

    fn payload_for(video: &Video) -> String {
        Task::FetchChapters {
            video_id: video.id.as_str().to_string(),
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

    fn run(task: &FetchChaptersTask, payload: &str) -> Result<(), String> {
        task.handle(payload, false).map_err(|e| e.to_string())
    }
}
