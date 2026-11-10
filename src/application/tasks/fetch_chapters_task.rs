use crate::domain::services::{ChapterFetcher, ChapterFetcherApi};
use crate::domain::task::Task;
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};
use std::sync::Arc;

/// Fetches one downloaded video's chapters, scheduled by the video sync
/// (`SyncVideoTask` via `VideoReconciler`) while its chapters are unsettled and
/// due (see the `late-video-data` capability). Best-effort: always succeeds,
/// so the queue never retries it.
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
    fn handle(&self, task: &Task, _is_last_attempt: bool) -> anyhow::Result<()> {
        let Task::FetchChapters { video_id } = task else {
            return Err(unexpected_task(task));
        };
        let video_id = VideoId::new(video_id.clone())?;
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
    use crate::domain::video::{
        ChaptersProbe, LateDataChecks, PlaybackPosition, Video, VideoChapter, YoutubeVideoId,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::repositories::youtube_chapter_repository::FakeChapterRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Duration, Utc};

    #[test]
    fn it_should_settle_chapters_found() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let chapter_repository =
            Arc::new(FakeChapterRepository::with_chapters(Ok(intro_chapters())));
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                chapter_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                chapters: Some(intro_chapters()),
                chapters_checks: LateDataChecks {
                    last_checked_at: Some(later()),
                    failed_attempts: 0,
                },
                chapters_settled: true,
                updated_at: later(),
                ..video
            }]
        );
        assert_eq!(
            *chapter_repository.calls.lock().unwrap(),
            vec![YT1_URL.to_string()]
        );
    }

    #[test]
    fn it_should_recheck_unsettled_empty_chapters() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let chapter_repository =
            Arc::new(FakeChapterRepository::with_chapters(Ok(intro_chapters())));
        let video = Video {
            chapters_checks: LateDataChecks {
                last_checked_at: Some(fixed_timestamp()),
                failed_attempts: 0,
            },
            ..downloaded_video().with_chapters(vec![], fixed_timestamp())
        };
        video_repository.save(&video).unwrap();
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                chapter_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                chapters: Some(intro_chapters()),
                chapters_checks: LateDataChecks {
                    last_checked_at: Some(later()),
                    failed_attempts: 0,
                },
                chapters_settled: true,
                updated_at: later(),
                ..video
            }]
        );
        assert_eq!(
            *chapter_repository.calls.lock().unwrap(),
            vec![YT1_URL.to_string()]
        );
    }

    #[test]
    fn it_should_keep_empty_chapters_unsettled_for_a_young_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let chapter_repository = Arc::new(FakeChapterRepository::with_chapters(Ok(vec![])));
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                chapter_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                chapters: Some(vec![]),
                chapters_checks: LateDataChecks {
                    last_checked_at: Some(later()),
                    failed_attempts: 0,
                },
                updated_at: later(),
                ..video
            }]
        );
    }

    #[test]
    fn it_should_settle_empty_chapters_of_an_old_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let chapter_repository = Arc::new(FakeChapterRepository::with_probe(Ok(ChaptersProbe {
            chapters: vec![],
            published_at: Some(later() - Duration::days(30)),
        })));
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                chapter_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                chapters: Some(vec![]),
                chapters_checks: LateDataChecks {
                    last_checked_at: Some(later()),
                    failed_attempts: 0,
                },
                chapters_settled: true,
                updated_at: later(),
                ..video
            }]
        );
    }

    #[test]
    fn it_should_count_a_failed_chapters_attempt() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let chapter_repository = Arc::new(FakeChapterRepository::with_chapters(Err(
            anyhow::anyhow!("Video unavailable"),
        )));
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                chapter_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                chapters_checks: LateDataChecks {
                    last_checked_at: Some(later()),
                    failed_attempts: 1,
                },
                updated_at: later(),
                ..video
            }]
        );
        assert_eq!(
            *chapter_repository.calls.lock().unwrap(),
            vec![YT1_URL.to_string()]
        );
    }

    #[test]
    fn it_should_settle_chapters_as_none_after_three_failed_checks_of_an_old_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let chapter_repository = Arc::new(FakeChapterRepository::with_chapters(Err(
            anyhow::anyhow!("Video unavailable"),
        )));
        let video = Video {
            created_at: later() - Duration::days(30),
            chapters_checks: LateDataChecks {
                last_checked_at: Some(later() - Duration::days(1)),
                failed_attempts: 2,
            },
            ..downloaded_video()
        };
        video_repository.save(&video).unwrap();
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                chapter_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                chapters: Some(vec![]),
                chapters_checks: LateDataChecks {
                    last_checked_at: Some(later()),
                    failed_attempts: 3,
                },
                chapters_settled: true,
                updated_at: later(),
                ..video
            }]
        );
        assert_eq!(
            *chapter_repository.calls.lock().unwrap(),
            vec![YT1_URL.to_string()]
        );
    }

    #[test]
    fn it_should_skip_settled_chapters() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let chapter_repository =
            Arc::new(FakeChapterRepository::with_chapters(Ok(intro_chapters())));
        let video = Video {
            chapters_settled: true,
            ..downloaded_video().with_chapters(vec![], fixed_timestamp())
        };
        video_repository.save(&video).unwrap();
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                chapter_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(
            *chapter_repository.calls.lock().unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn it_should_skip_if_the_video_is_gone() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let chapter_repository =
            Arc::new(FakeChapterRepository::with_chapters(Ok(intro_chapters())));
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                chapter_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &task_for(&downloaded_video()));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            *chapter_repository.calls.lock().unwrap(),
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
        let chapter_repository = Arc::new({
            let video_repository = video_repository.clone();
            let played = played.clone();
            FakeChapterRepository::with_chapters(Ok(intro_chapters()))
                .with_on_fetch(move || video_repository.update(&played).unwrap())
        });
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                chapter_repository,
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                chapters: Some(intro_chapters()),
                chapters_checks: LateDataChecks {
                    last_checked_at: Some(later()),
                    failed_attempts: 0,
                },
                chapters_settled: true,
                updated_at: later(),
                ..played
            }]
        );
    }

    #[test]
    fn it_should_discard_the_chapters_check_if_the_video_was_deleted_meanwhile() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let chapter_repository = Arc::new({
            let video_repository = video_repository.clone();
            let deleted_id = video.id.clone();
            FakeChapterRepository::with_chapters(Ok(intro_chapters()))
                .with_on_fetch(move || video_repository.delete(&deleted_id).unwrap())
        });
        let task = FetchChaptersTask::new(
            video_repository.clone(),
            Arc::new(ChapterFetcher::new(
                video_repository.clone(),
                chapter_repository.clone(),
                Arc::new(FixedClock(later())),
            )),
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            *chapter_repository.calls.lock().unwrap(),
            vec![YT1_URL.to_string()]
        );
    }

    const YT1_URL: &str = "https://www.youtube.com/watch?v=yt1";

    /// `yt1`, downloaded with its chapters never checked.
    fn downloaded_video() -> Video {
        Video::create(
            YoutubeVideoId::new("yt1").unwrap(),
            "My Video",
            fixed_timestamp(),
        )
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

    fn task_for(video: &Video) -> Task {
        Task::FetchChapters {
            video_id: video.id.as_str().to_string(),
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn later() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_600, 0).unwrap()
    }

    fn run(task: &FetchChaptersTask, queued: &Task) -> Result<(), String> {
        task.handle(queued, false).map_err(|e| e.to_string())
    }
}
