use crate::domain::services::{ThumbnailFetcher, ThumbnailFetcherApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};
use std::sync::Arc;

/// Fetches one video's thumbnail ahead of its download, scheduled by
/// the video sync (`SyncVideoTask` via `VideoReconciler`) and by the
/// reconcilers' missing-thumbnail recovery.
/// Best-effort: always succeeds, so the queue never retries a video that
/// simply has no thumbnail. The next reconcile pass reschedules it instead.
pub struct FetchThumbnailTask {
    thumbnail_fetcher: Arc<ThumbnailFetcher>,
}

impl FetchThumbnailTask {
    pub fn new(thumbnail_fetcher: Arc<ThumbnailFetcher>) -> Self {
        Self { thumbnail_fetcher }
    }
}

impl TaskHandler for FetchThumbnailTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::FetchThumbnail { video_id } = task else {
            return Err(unexpected_task(task));
        };
        self.thumbnail_fetcher
            .fetch(&VideoId::new(video_id.clone())?)
            .map(TaskOutcome::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::services::VideoOwnerFinder;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::video::{
        EpisodeNumber, FetchedThumbnail, RetryAttempts, StepState, StepStatus, Video,
        YoutubeVideoId,
    };
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, SqliteChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, SqliteChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_thumbnail_repository::FakeThumbnailRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::staging_area::StagingArea;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Duration, Utc};
    use std::path::PathBuf;

    #[test]
    fn it_should_place_a_prefetched_thumbnail_as_the_episode_image() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let thumbnail_repository = Arc::new(FakeThumbnailRepository::with_result(Some(
            FetchedThumbnail {
                path: PathBuf::from("yt1.jpg"),
                published_at: Some(published_at()),
            },
        )));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = FetchThumbnailTask::new(Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            thumbnail_repository.clone(),
            video_file_repository.clone(),
            Arc::new(FixedClock(later())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                ..video
                    .clone()
                    .with_episode_number(EpisodeNumber::from_columns(2026, 31501).unwrap(), later())
                    .with_thumbnail("Season 2026/S2026E031501 - My Video.jpg", later())
            }]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                PathBuf::from(format!("/videos/.downloads/{}/yt1.jpg", video.id.as_str())),
                PathBuf::from(
                    "/videos/creators/somechannel/Season 2026/S2026E031501 - My Video.jpg"
                ),
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
            *thumbnail_repository.calls.lock().unwrap(),
            vec![thumbnail_call(&video)]
        );
    }

    #[test]
    fn it_should_skip_if_video_already_has_a_thumbnail() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let thumbnail_repository = Arc::new(FakeThumbnailRepository::default());
        let video = my_video().with_thumbnail("My Video/My Video.jpg", fixed_timestamp());
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = FetchThumbnailTask::new(Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            thumbnail_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(later())),
            Arc::new(StagingArea::new(
                Arc::new(FakeVideoFileRepository::default()),
                "/videos",
            )),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(*thumbnail_repository.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_fetch_a_thumbnail_for_an_excluded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let thumbnail_repository = Arc::new(FakeThumbnailRepository::with_result(Some(
            FetchedThumbnail {
                path: std::path::PathBuf::from("My Video.jpg"),
                published_at: None,
            },
        )));
        let video = my_video().mark_unavailable(fixed_timestamp());
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = FetchThumbnailTask::new(Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            thumbnail_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(later())),
            Arc::new(StagingArea::new(
                Arc::new(FakeVideoFileRepository::default()),
                "/videos",
            )),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(*thumbnail_repository.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_fetch_a_thumbnail_for_an_errored_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let thumbnail_repository = Arc::new(FakeThumbnailRepository::with_result(Some(
            FetchedThumbnail {
                path: std::path::PathBuf::from("My Video.jpg"),
                published_at: None,
            },
        )));
        let video = my_video().mark_download_failed("HTTP Error 503", fixed_timestamp());
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = FetchThumbnailTask::new(Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            thumbnail_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(later())),
            Arc::new(StagingArea::new(
                Arc::new(FakeVideoFileRepository::default()),
                "/videos",
            )),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(*thumbnail_repository.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_if_video_no_longer_exists() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let thumbnail_repository = Arc::new(FakeThumbnailRepository::default());
        let task = FetchThumbnailTask::new(Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            thumbnail_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(later())),
            Arc::new(StagingArea::new(
                Arc::new(FakeVideoFileRepository::default()),
                "/videos",
            )),
        )));

        let result = run(&task, &task_for("missing"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(*thumbnail_repository.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_if_no_playlist_or_channel_holds_the_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let thumbnail_repository = Arc::new(FakeThumbnailRepository::with_result(Some(
            FetchedThumbnail {
                path: PathBuf::from("yt1.jpg"),
                published_at: Some(published_at()),
            },
        )));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        let task = FetchThumbnailTask::new(Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            thumbnail_repository.clone(),
            video_file_repository.clone(),
            Arc::new(FixedClock(later())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(*thumbnail_repository.calls.lock().unwrap(), vec![]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_discard_the_thumbnail_if_the_video_was_deleted_meanwhile() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let thumbnail_repository = Arc::new({
            let video_repository = video_repository.clone();
            let deleted_id = video.id.clone();
            FakeThumbnailRepository::with_result(Some(FetchedThumbnail {
                path: PathBuf::from("yt1.jpg"),
                published_at: Some(published_at()),
            }))
            .with_on_fetch(move || video_repository.delete(&deleted_id).unwrap())
        });
        let task = FetchThumbnailTask::new(Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            thumbnail_repository.clone(),
            video_file_repository.clone(),
            Arc::new(FixedClock(later())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from(format!(
                "/videos/.downloads/{}",
                video.id.as_str()
            ))]
        );
        assert_eq!(
            *thumbnail_repository.calls.lock().unwrap(),
            vec![thumbnail_call(&video)]
        );
    }

    #[test]
    fn it_should_discard_the_thumbnail_if_the_video_was_given_one_meanwhile() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let thumbnail_repository = Arc::new({
            let video_repository = video_repository.clone();
            let video_id = video.id.clone();
            FakeThumbnailRepository::with_result(Some(FetchedThumbnail {
                path: PathBuf::from("yt1.jpg"),
                published_at: Some(published_at()),
            }))
            .with_on_fetch(move || {
                let stored = video_repository.find(&video_id).unwrap().unwrap();
                video_repository
                    .update(&stored.with_thumbnail(DOWNLOADED_THUMBNAIL, later()))
                    .unwrap()
            })
        });
        let task = FetchThumbnailTask::new(Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            thumbnail_repository.clone(),
            video_file_repository.clone(),
            Arc::new(FixedClock(later())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..video.clone().with_thumbnail(DOWNLOADED_THUMBNAIL, later())
            }]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from(format!(
                "/videos/.downloads/{}",
                video.id.as_str()
            ))]
        );
        assert_eq!(
            *thumbnail_repository.calls.lock().unwrap(),
            vec![thumbnail_call(&video)]
        );
    }

    #[test]
    fn it_should_schedule_the_next_attempt_if_no_thumbnail_is_obtained() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let thumbnail_repository = Arc::new(FakeThumbnailRepository::failing());
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = FetchThumbnailTask::new(Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            thumbnail_repository.clone(),
            video_file_repository.clone(),
            Arc::new(FixedClock(later())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: later() + Duration::minutes(10),
                reason: "fake thumbnail fetch error".to_string(),
            })
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                thumbnail: StepState {
                    status: StepStatus::Retrying,
                    attempts: RetryAttempts {
                        failed: 1,
                        last_failed_at: Some(later()),
                    },
                },
                updated_at: later(),
                ..video.clone()
            }]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from(format!(
                "/videos/.downloads/{}",
                video.id.as_str()
            ))]
        );
        assert_eq!(
            *thumbnail_repository.calls.lock().unwrap(),
            vec![thumbnail_call(&video)]
        );
    }

    #[test]
    fn it_should_fail_the_thumbnail_after_the_eleventh_failed_attempt() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video = Video {
            thumbnail: StepState {
                status: StepStatus::Retrying,
                attempts: RetryAttempts {
                    failed: 10,
                    last_failed_at: Some(fixed_timestamp() - Duration::days(30)),
                },
            },
            ..my_video()
        };
        video_repository.insert(&video).unwrap();
        seed_channel_video(&db, &video);
        let task = thumbnail_task(
            &db,
            video_repository.clone(),
            Arc::new(FakeThumbnailRepository::failing()),
            video_file_repository,
        );

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                thumbnail: StepState {
                    status: StepStatus::Failed,
                    attempts: RetryAttempts {
                        failed: 11,
                        last_failed_at: Some(later()),
                    },
                },
                updated_at: later(),
                ..video
            }]
        );
    }

    #[test]
    fn it_should_mark_the_thumbnail_done_once_fetched() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = Video {
            thumbnail: StepState {
                status: StepStatus::Retrying,
                attempts: RetryAttempts {
                    failed: 2,
                    last_failed_at: Some(fixed_timestamp() - Duration::days(30)),
                },
            },
            ..my_video()
        };
        video_repository.insert(&video).unwrap();
        seed_channel_video(&db, &video);
        let task = thumbnail_task(
            &db,
            video_repository.clone(),
            Arc::new(FakeThumbnailRepository::with_result(Some(
                FetchedThumbnail {
                    path: PathBuf::from("yt1.jpg"),
                    published_at: Some(published_at()),
                },
            ))),
            Arc::new(FakeVideoFileRepository::default()),
        );

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                thumbnail_filename: Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                thumbnail: StepState {
                    status: StepStatus::Done,
                    attempts: RetryAttempts::default(),
                },
                episode_number: Some(EpisodeNumber::from_columns(2026, 31501).unwrap()),
                updated_at: later(),
                ..video
            }]
        );
    }

    /// A thumbnail a download placed while the fetch was in flight.
    const DOWNLOADED_THUMBNAIL: &str = "Season 2026/S2026E031501 - My Video.jpg";

    /// Stores `my_channel()` holding `video`.
    fn seed_channel_video(db: &TestDatabase, video: &Video) {
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    fn thumbnail_task(
        db: &TestDatabase,
        video_repository: Arc<SqliteVideoRepository>,
        thumbnail_repository: Arc<FakeThumbnailRepository>,
        video_file_repository: Arc<FakeVideoFileRepository>,
    ) -> FetchThumbnailTask {
        FetchThumbnailTask::new(Arc::new(ThumbnailFetcher::new(
            video_repository,
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            thumbnail_repository,
            video_file_repository.clone(),
            Arc::new(FixedClock(later())),
            Arc::new(StagingArea::new(video_file_repository, "/videos")),
        )))
    }

    fn my_channel_id() -> ChannelHandle {
        ChannelHandle::new("@somechannel").unwrap()
    }

    /// The channel holding `my_video()`, its show folder
    /// `/videos/creators/somechannel`.
    fn my_channel() -> Channel {
        Channel::create(
            my_channel_id(),
            "Some Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            ReconcileInterval::default_interval(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    fn my_video() -> Video {
        Video::create(
            YoutubeVideoId::new("yt1").unwrap(),
            "My Video",
            fixed_timestamp(),
        )
    }

    /// One call as `FakeThumbnailRepository` records it, for the `yt1`
    /// video, staged under its id.
    fn thumbnail_call(video: &Video) -> (String, PathBuf) {
        (
            "https://www.youtube.com/watch?v=yt1".to_string(),
            PathBuf::from(format!("/videos/.downloads/{}", video.id.as_str())),
        )
    }

    fn task_for(video_id: &str) -> Task {
        Task::FetchThumbnail {
            video_id: video_id.to_string(),
        }
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

    fn run(task: &FetchThumbnailTask, queued: &Task) -> Result<TaskOutcome, String> {
        task.handle(queued).map_err(|e| e.to_string())
    }
}
