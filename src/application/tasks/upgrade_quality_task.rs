use crate::domain::services::{QualityUpgrader, QualityUpgraderApi};
use crate::domain::shared::Quality;
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::path::Path;
use std::sync::Arc;

/// Checks one young downloaded video's quality and upgrades it in place when
/// YouTube now offers better, scheduled by the video sync (`SyncVideoTask`
/// via `VideoReconciler`) while its quality is unsettled and due.
/// Best-effort: always succeeds, so the queue never retries it.
pub struct UpgradeQualityTask {
    quality_upgrader: Arc<QualityUpgrader>,
}

impl UpgradeQualityTask {
    pub fn new(quality_upgrader: Arc<QualityUpgrader>) -> Self {
        Self { quality_upgrader }
    }
}

impl TaskHandler for UpgradeQualityTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let (video_id, quality, output_dir) = Task::decode_upgrade_quality_payload(payload)?;
        self.quality_upgrader.upgrade(
            &VideoRecordId::new(video_id)?,
            Quality::new(quality)?,
            Path::new(&output_dir),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::video::{
        EpisodeNumber, LateDataChecks, PlaybackPosition, Video, VideoDownloaded, VideoId,
    };
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::{
        DownloadedVideo, FakeVideoDownloaderRepository, QualityProbe,
    };
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Duration, Utc};
    use std::path::PathBuf;

    #[test]
    fn it_should_replace_the_file_with_a_higher_quality_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::succeeding_with(DownloadedVideo {
                height: Some(1080),
                ..staged_download()
            })
            .with_quality_probe(Ok(QualityProbe {
                degraded: false,
                height: Some(1080),
                published_at: Some(published_yesterday()),
            })),
        );
        let video = sd_video();
        video_repository.save(&video).unwrap();
        let task = UpgradeQualityTask::new(Arc::new(QualityUpgrader::new(
            video_repository.clone(),
            video_downloader_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            "/videos",
        )));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                downloaded_height: Some(1080),
                quality_checks: LateDataChecks {
                    last_checked_at: Some(now()),
                    failed_attempts: 0,
                },
                quality_settled: true,
                updated_at: now(),
                ..video.clone()
            }]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                staging(&video).join("yt1.mp4"),
                PathBuf::from(OUTPUT_DIR).join(FILENAME),
            )]
        );
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![staging(&video)]
        );
        assert_eq!(
            *video_downloader_repository.quality_calls.lock().unwrap(),
            vec![(YT1_URL.to_string(), Quality::High)]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![ScheduledEvent {
                id: 1,
                event_type: "video_downloaded".to_string(),
                payload: DomainEvent::VideoDownloaded(VideoDownloaded {
                    video_id: video.id.as_str().to_string(),
                    output_dir: OUTPUT_DIR.to_string(),
                    folder: "Season 2026".to_string(),
                })
                .payload()
                .to_string(),
                retries: 0,
                created_at: now(),
                updated_at: now(),
                last_error: None,
            }]
        );
    }

    #[test]
    fn it_should_count_a_degraded_upgrade_as_a_failed_check() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::succeeding_with(DownloadedVideo {
                height: Some(720),
                sabr_notice: Some(
                    "YouTube may have enabled the SABR-only streaming experiment".to_string(),
                ),
                ..staged_download()
            })
            .with_quality_probe(Ok(QualityProbe {
                degraded: false,
                height: Some(1080),
                published_at: Some(now() - Duration::days(8)),
            })),
        );
        let video = sd_video();
        video_repository.save(&video).unwrap();
        let task = UpgradeQualityTask::new(Arc::new(QualityUpgrader::new(
            video_repository.clone(),
            video_downloader_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            "/videos",
        )));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                downloaded_height: Some(720),
                quality_checks: LateDataChecks {
                    last_checked_at: Some(now()),
                    failed_attempts: 1,
                },
                updated_at: now(),
                ..video.clone()
            }]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                staging(&video).join("yt1.mp4"),
                PathBuf::from(OUTPUT_DIR).join(FILENAME),
            )]
        );
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![staging(&video)]
        );
        assert_eq!(
            *video_downloader_repository.quality_calls.lock().unwrap(),
            vec![(YT1_URL.to_string(), Quality::High)]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![ScheduledEvent {
                id: 1,
                event_type: "video_downloaded".to_string(),
                payload: DomainEvent::VideoDownloaded(VideoDownloaded {
                    video_id: video.id.as_str().to_string(),
                    output_dir: OUTPUT_DIR.to_string(),
                    folder: "Season 2026".to_string(),
                })
                .payload()
                .to_string(),
                retries: 0,
                created_at: now(),
                updated_at: now(),
                last_error: None,
            }]
        );
    }

    #[test]
    fn it_should_record_the_check_if_nothing_better_is_available() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::new(true).with_quality_probe(Ok(QualityProbe {
                degraded: false,
                height: Some(360),
                published_at: Some(published_yesterday()),
            })),
        );
        let video = sd_video();
        video_repository.save(&video).unwrap();
        let task = UpgradeQualityTask::new(Arc::new(QualityUpgrader::new(
            video_repository.clone(),
            video_downloader_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            "/videos",
        )));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                quality_checks: LateDataChecks {
                    last_checked_at: Some(now()),
                    failed_attempts: 0,
                },
                updated_at: now(),
                ..video.clone()
            }]
        );
        assert_eq!(*video_downloader_repository.calls.lock().unwrap(), vec![]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_count_a_failed_attempt_if_the_probe_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::new(true)
                .with_quality_probe(Err(anyhow::anyhow!("HTTP Error 429: Too Many Requests"))),
        );
        let video = sd_video();
        video_repository.save(&video).unwrap();
        let task = UpgradeQualityTask::new(Arc::new(QualityUpgrader::new(
            video_repository.clone(),
            video_downloader_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            "/videos",
        )));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                quality_checks: LateDataChecks {
                    last_checked_at: Some(now()),
                    failed_attempts: 1,
                },
                updated_at: now(),
                ..video.clone()
            }]
        );
        assert_eq!(*video_downloader_repository.calls.lock().unwrap(), vec![]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_keep_the_file_if_the_redownload_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::with_failed_stderr("HTTP Error 403: Forbidden")
                .with_quality_probe(Ok(QualityProbe {
                    degraded: false,
                    height: Some(1080),
                    published_at: Some(published_yesterday()),
                })),
        );
        let video = sd_video();
        video_repository.save(&video).unwrap();
        let task = UpgradeQualityTask::new(Arc::new(QualityUpgrader::new(
            video_repository.clone(),
            video_downloader_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            "/videos",
        )));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                quality_checks: LateDataChecks {
                    last_checked_at: Some(now()),
                    failed_attempts: 1,
                },
                updated_at: now(),
                ..video.clone()
            }]
        );
        assert_eq!(
            *video_downloader_repository.calls.lock().unwrap(),
            vec![(YT1_URL.to_string(), Quality::High, staging(&video))]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_keep_the_file_if_the_redownload_is_no_better() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::succeeding_with(DownloadedVideo {
                height: Some(240),
                ..staged_download()
            })
            .with_quality_probe(Ok(QualityProbe {
                degraded: false,
                height: Some(1080),
                published_at: Some(published_yesterday()),
            })),
        );
        let video = sd_video();
        video_repository.save(&video).unwrap();
        let task = UpgradeQualityTask::new(Arc::new(QualityUpgrader::new(
            video_repository.clone(),
            video_downloader_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            "/videos",
        )));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                quality_checks: LateDataChecks {
                    last_checked_at: Some(now()),
                    failed_attempts: 1,
                },
                updated_at: now(),
                ..video.clone()
            }]
        );
        assert_eq!(
            *video_downloader_repository.calls.lock().unwrap(),
            vec![(YT1_URL.to_string(), Quality::High, staging(&video))]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_keep_the_file_if_the_redownload_is_no_taller() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::succeeding_with(DownloadedVideo {
                height: Some(360),
                ..staged_download()
            })
            .with_quality_probe(Ok(QualityProbe {
                degraded: false,
                height: Some(1080),
                published_at: Some(published_yesterday()),
            })),
        );
        let video = sd_video();
        video_repository.save(&video).unwrap();
        let task = UpgradeQualityTask::new(Arc::new(QualityUpgrader::new(
            video_repository.clone(),
            video_downloader_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            "/videos",
        )));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                quality_checks: LateDataChecks {
                    last_checked_at: Some(now()),
                    failed_attempts: 1,
                },
                updated_at: now(),
                ..video.clone()
            }]
        );
        assert_eq!(
            *video_downloader_repository.calls.lock().unwrap(),
            vec![(YT1_URL.to_string(), Quality::High, staging(&video))]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_settle_quality_once_the_video_is_old() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::new(true).with_quality_probe(Ok(QualityProbe {
                degraded: false,
                height: Some(360),
                published_at: Some(now() - Duration::days(8)),
            })),
        );
        let video = sd_video();
        video_repository.save(&video).unwrap();
        let task = UpgradeQualityTask::new(Arc::new(QualityUpgrader::new(
            video_repository.clone(),
            video_downloader_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            "/videos",
        )));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                quality_checks: LateDataChecks {
                    last_checked_at: Some(now()),
                    failed_attempts: 0,
                },
                quality_settled: true,
                updated_at: now(),
                ..video.clone()
            }]
        );
        assert_eq!(*video_downloader_repository.calls.lock().unwrap(), vec![]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_count_a_degraded_probe_as_a_failed_check() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::new(true).with_quality_probe(Ok(QualityProbe {
                degraded: true,
                height: Some(360),
                published_at: Some(now() - Duration::days(30)),
            })),
        );
        let video = sd_video();
        video_repository.save(&video).unwrap();
        let task = UpgradeQualityTask::new(Arc::new(QualityUpgrader::new(
            video_repository.clone(),
            video_downloader_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            "/videos",
        )));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                quality_checks: LateDataChecks {
                    last_checked_at: Some(now()),
                    failed_attempts: 1,
                },
                updated_at: now(),
                ..video.clone()
            }]
        );
        assert_eq!(*video_downloader_repository.calls.lock().unwrap(), vec![]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_the_upgrade_if_the_video_was_reset_meanwhile() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video = sd_video();
        video_repository.save(&video).unwrap();
        let reset = video.clone().reset_for_redownload(now());
        let video_downloader_repository = Arc::new({
            let video_repository = video_repository.clone();
            let reset = reset.clone();
            FakeVideoDownloaderRepository::succeeding_with(DownloadedVideo {
                height: Some(1080),
                ..staged_download()
            })
            .with_quality_probe(Ok(QualityProbe {
                degraded: false,
                height: Some(1080),
                published_at: Some(published_yesterday()),
            }))
            .with_on_download(move || video_repository.update(&reset).unwrap())
        });
        let task = UpgradeQualityTask::new(Arc::new(QualityUpgrader::new(
            video_repository.clone(),
            video_downloader_repository,
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            "/videos",
        )));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![reset]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_the_upgrade_if_the_video_was_redownloaded_meanwhile() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video = sd_video();
        video_repository.save(&video).unwrap();
        let redownloaded = video.clone().mark_downloaded(
            Quality::High,
            "Season 2026/S2026E031501 - My Renamed Video.mp4",
            Some("Season 2026/S2026E031501 - My Renamed Video.jpg".to_string()),
            Some(600),
            now(),
        );
        let video_downloader_repository = Arc::new({
            let video_repository = video_repository.clone();
            let redownloaded = redownloaded.clone();
            FakeVideoDownloaderRepository::succeeding_with(DownloadedVideo {
                height: Some(1080),
                ..staged_download()
            })
            .with_quality_probe(Ok(QualityProbe {
                degraded: false,
                height: Some(1080),
                published_at: Some(published_yesterday()),
            }))
            .with_on_download(move || video_repository.update(&redownloaded).unwrap())
        });
        let task = UpgradeQualityTask::new(Arc::new(QualityUpgrader::new(
            video_repository.clone(),
            video_downloader_repository,
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            "/videos",
        )));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![redownloaded]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_a_video_with_settled_quality() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::new(true).with_quality_probe(Ok(QualityProbe {
                degraded: false,
                height: Some(1080),
                published_at: Some(published_yesterday()),
            })),
        );
        let video = Video {
            quality_settled: true,
            ..sd_video()
        };
        video_repository.save(&video).unwrap();
        let task = UpgradeQualityTask::new(Arc::new(QualityUpgrader::new(
            video_repository.clone(),
            video_downloader_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            "/videos",
        )));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(
            *video_downloader_repository.quality_calls.lock().unwrap(),
            vec![]
        );
        assert_eq!(*video_downloader_repository.calls.lock().unwrap(), vec![]);
    }

    const YT1_URL: &str = "https://www.youtube.com/watch?v=yt1";
    const OUTPUT_DIR: &str = "/videos/My Channel";
    const FILENAME: &str = "Season 2026/S2026E031501 - My Video.mp4";

    /// `yt1`, published yesterday, downloaded at 360p a day ago as episode
    /// `2026`/`031501` with its subtitles and chapters settled and half
    /// watched; its `high` quality unsettled.
    fn sd_video() -> Video {
        let downloaded_at = now() - Duration::days(1);
        Video {
            downloaded_height: Some(360),
            quality_checks: LateDataChecks {
                last_checked_at: Some(downloaded_at),
                failed_attempts: 0,
            },
            chapters_settled: true,
            ..Video::create(VideoId::new("yt1").unwrap(), "My Video", downloaded_at)
                .with_episode_number(
                    EpisodeNumber::from_columns(2026, 31501).unwrap(),
                    downloaded_at,
                )
                .mark_downloaded(
                    Quality::High,
                    FILENAME,
                    Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                    Some(600),
                    downloaded_at,
                )
                .mark_subtitles_fetched("Season 2026/S2026E031501 - My Video.en.srt", downloaded_at)
                .with_chapters(vec![], downloaded_at)
                .update_watch_state(
                    PlaybackPosition::new(300).unwrap(),
                    None,
                    false,
                    downloaded_at,
                )
        }
    }

    /// The re-download `yt-dlp` staged for `yt1`.
    fn staged_download() -> DownloadedVideo {
        DownloadedVideo {
            media_path: PathBuf::from("yt1.mp4"),
            thumbnail_path: Some(PathBuf::from("yt1.jpg")),
            duration_seconds: Some(600),
            published_at: Some(published_yesterday()),
            sabr_notice: None,
            chapters: Some(vec![]),
            height: None,
        }
    }

    fn staging(video: &Video) -> PathBuf {
        PathBuf::from(format!("/videos/.downloads/{}", video.id.as_str()))
    }

    fn payload_for(video: &Video) -> String {
        Task::UpgradeQuality {
            video_id: video.id.as_str().to_string(),
            quality: "high".to_string(),
            output_dir: OUTPUT_DIR.to_string(),
        }
        .payload()
        .to_string()
    }

    fn published_yesterday() -> DateTime<Utc> {
        now() - Duration::days(1)
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-03-16T10:00:00Z")
            .unwrap()
            .to_utc()
    }

    fn run(task: &UpgradeQualityTask, payload: &str) -> Result<(), String> {
        task.handle(payload, false).map_err(|e| e.to_string())
    }
}
