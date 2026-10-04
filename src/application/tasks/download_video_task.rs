use crate::domain::services::{VideoDownloader, VideoDownloaderApi};
use crate::domain::shared::Quality;
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::path::Path;

/// Downloads one video via `yt-dlp`, scheduled by
/// `subscribers::download_video_on_video_added_to_playlist`/
/// `..._to_channel` whenever a new video shows up in a tracked container.
pub struct DownloadVideoTask {
    video_downloader: VideoDownloader,
}

impl DownloadVideoTask {
    pub fn new(video_downloader: VideoDownloader) -> Self {
        Self { video_downloader }
    }
}

impl TaskHandler for DownloadVideoTask {
    fn handle(&self, payload: &str, is_last_attempt: bool) -> anyhow::Result<()> {
        let (video_id, quality, output_dir) = Task::decode_download_video_payload(payload)?;
        let Ok(video_id) = VideoRecordId::new(video_id) else {
            return Ok(());
        };
        let Ok(quality) = Quality::new(quality) else {
            return Ok(());
        };
        self.video_downloader
            .download(video_id, quality, Path::new(&output_dir), is_last_attempt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tasks::log_capture::captured_log_messages;
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::video::Video;
    use crate::domain::video::{VideoId, VideoStatus};
    use crate::domain::video_metadata::{VideoMetadata, render_movie_nfo};
    use crate::infrastructure::repositories::filesystem_video_file_repository::{
        FakeVideoFileRepository, FilesystemVideoFileRepository, VideoFileRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::{
        MOVIE_NFO_FILENAME, SqliteVideoMetadataRepository, VideoMetadataRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::{
        FailingOnceYoutubeMetadataRepository, FakeYoutubeMetadataRepository, YoutubeMetadata,
        YoutubeMetadataRepository,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::{
        FAKE_FRESH_FOLDER, FakeVideoDownloaderRepository, VideoDownloaderRepository,
        YtDlpVideoDownloaderRepository,
    };
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    #[cfg(unix)]
    use crate::infrastructure::shared::ytdlp::test_support::{FakeYtDlp, unique_temp_dir};
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn it_should_download_the_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.start_download(fixed_timestamp()).mark_downloaded(
                Quality::High,
                "fake-output/fake-output.mp4",
                None,
                None,
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_publish_that_the_video_was_downloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoDownloaded {
                    video_id: video.id.as_str().to_string(),
                    output_dir: "/videos/my-playlist".to_string(),
                    folder: FAKE_FRESH_FOLDER.to_string(),
                }
            )]
        );
    }

    #[test]
    fn it_should_record_the_sync_time() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                status: VideoStatus::Downloaded,
                quality: Some(Quality::High),
                filename: Some("fake-output/fake-output.mp4".to_string()),
                synced_at: Some(fixed_timestamp()),
                ..video
            }]
        );
    }

    #[test]
    fn it_should_retry_a_failed_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(false)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(
            result,
            Err(format!("yt-dlp failed to download video {}", video.id))
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_errored_retrying(fixed_timestamp())
            ]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_record_the_download_error() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::with_failed_stderr(
                "HTTP Error 403: Forbidden",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Err("HTTP Error 403: Forbidden".to_string()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_errored_retrying(fixed_timestamp())
            ]
        );
    }

    #[test]
    fn it_should_still_record_a_degraded_sabr_download_as_downloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with_sabr(
                "SABR-only streaming experiment",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.start_download(fixed_timestamp()).mark_downloaded(
                Quality::High,
                "fake-output/fake-output.mp4",
                None,
                None,
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_fail_and_retry_a_sabr_failure_exactly_as_a_normal_failure() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::with_failed_stderr_and_sabr(
                "HTTP Error 403: Forbidden",
                "SABR-only streaming experiment",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Err("HTTP Error 403: Forbidden".to_string()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_errored_retrying(fixed_timestamp())
            ]
        );
    }

    #[test]
    fn it_should_log_a_sabr_event_on_a_degraded_successful_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with_sabr(
                "SABR-only streaming experiment",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let logs = captured_log_messages(|| {
            let _ = run(&task, &payload_for(video.id.as_str()), false);
        });

        assert!(
            logs.iter().any(
                |message| message.contains("SABR-only streaming experiment reported by yt-dlp")
            )
        );
    }

    #[test]
    fn it_should_log_a_sabr_event_on_a_sabr_download_failure() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::with_failed_stderr_and_sabr(
                "HTTP Error 403: Forbidden",
                "SABR-only streaming experiment",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let logs = captured_log_messages(|| {
            let _ = run(&task, &payload_for(video.id.as_str()), false);
        });

        assert!(
            logs.iter().any(
                |message| message.contains("SABR-only streaming experiment reported by yt-dlp")
            )
        );
    }

    #[test]
    fn it_should_not_log_a_sabr_event_on_a_clean_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let logs = captured_log_messages(|| {
            let _ = run(&task, &payload_for(video.id.as_str()), false);
        });

        assert!(
            !logs
                .iter()
                .any(|message| message.contains("SABR-only streaming experiment reported"))
        );
    }

    #[test]
    fn it_should_mark_errored_after_last_attempt() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(false)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), true);

        assert_eq!(
            result,
            Err(format!("yt-dlp failed to download video {}", video.id))
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_errored(fixed_timestamp())
            ]
        );
    }

    #[test]
    fn it_should_exclude_a_permanently_unavailable_video_without_dead_lettering_on_the_last_attempt()
     {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::with_failed_stderr("Video unavailable")
                    .with_diagnosed_reason(
                        "It was blocked due to the claimed content by Mediatoon.",
                    ),
            ),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), true);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_excluded(fixed_timestamp())
            ]
        );
    }

    #[test]
    fn it_should_exclude_a_video_blocked_by_claimed_content() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::with_failed_stderr("Video unavailable")
                    .with_diagnosed_reason(
                        "It was blocked due to the claimed content by Mediatoon.",
                    ),
            ),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_excluded(fixed_timestamp())
            ]
        );
    }

    #[test]
    fn it_should_exclude_a_members_only_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(false).with_diagnosed_reason(
                "Join this channel to get access to members-only content like this video, and other exclusive perks.",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_excluded(fixed_timestamp())
            ]
        );
    }

    #[test]
    fn it_should_not_exclude_a_bare_video_unavailable() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::with_failed_stderr(
                "Video unavailable",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Err("Video unavailable".to_string()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_errored_retrying(fixed_timestamp())
            ]
        );
    }

    #[test]
    fn it_should_treat_a_failed_diagnostic_probe_as_undetermined() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(false).with_diagnose_error()),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(
            result,
            Err(format!("yt-dlp failed to download video {}", video.id))
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_errored_retrying(fixed_timestamp())
            ]
        );
    }

    #[test]
    fn it_should_skip_if_video_is_gone() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(my_video().id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(*downloader.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_the_download_of_an_already_downloaded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video().mark_downloaded(
            Quality::High,
            "My Video/My Video.mp4",
            None,
            None,
            fixed_timestamp(),
        );
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(*downloader.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_the_download_of_an_excluded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video().mark_excluded(fixed_timestamp());
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(*downloader.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_the_download_of_an_errored_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video().mark_errored(fixed_timestamp());
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(*downloader.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_remove_the_folder_if_video_deleted_during_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video = my_video();
        video_repository.save(&video).unwrap();
        let deleting_repository = video_repository.clone();
        let deleted_id = video.id.clone();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::new(true).with_on_download(move || {
                    deleting_repository.delete(&deleted_id).unwrap();
                }),
            ),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/my-playlist"),
                "fake-output".to_string()
            )]
        );
    }

    #[test]
    fn it_should_remove_the_fresh_folder_if_download_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(false)),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(
            result,
            Err(format!("yt-dlp failed to download video {}", video.id))
        );
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/my-playlist"),
                FAKE_FRESH_FOLDER.to_string()
            )]
        );
    }

    #[test]
    fn it_should_keep_the_reused_folder_if_download_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video = my_video().with_thumbnail("My Video/My Video.jpg", fixed_timestamp());
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(false)),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(
            result,
            Err(format!("yt-dlp failed to download video {}", video.id))
        );
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_use_the_sanitized_title_as_filename() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = Video::create(
            VideoId::new("yt1").unwrap(),
            "My: Messy / Title?",
            fixed_timestamp(),
        );
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            *downloader.calls.lock().unwrap(),
            vec![download_call(
                "My- Messy - Title-",
                "/videos/my-playlist",
                Some(FAKE_FRESH_FOLDER)
            )]
        );
    }

    #[test]
    fn it_should_download_into_the_payload_output_dir() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(
            &task,
            &Task::DownloadVideo {
                video_id: video.id.as_str().to_string(),
                quality: "high".to_string(),
                output_dir: "/videos/a/b/c".to_string(),
            }
            .payload()
            .to_string(),
            false,
        );

        assert_eq!(result, Ok(()));
        assert_eq!(
            *downloader.calls.lock().unwrap(),
            vec![download_call(
                "My Video",
                "/videos/a/b/c",
                Some(FAKE_FRESH_FOLDER)
            )]
        );
    }

    #[test]
    fn it_should_reuse_the_existing_video_folder() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video().with_thumbnail("My Video/My Video.jpg", fixed_timestamp());
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            *downloader.calls.lock().unwrap(),
            vec![download_call(
                "My Video",
                "/videos/my-playlist",
                Some("My Video")
            )]
        );
    }

    #[test]
    fn it_should_download_into_a_fresh_folder_without_thumbnail() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            *downloader.calls.lock().unwrap(),
            vec![download_call(
                "My Video",
                "/videos/my-playlist",
                Some(FAKE_FRESH_FOLDER)
            )]
        );
    }

    #[test]
    fn it_should_skip_if_invalid_video_id_provided() {
        let result = run(&any_task(), &payload_for(""), false);

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let result = run(&any_task(), "not json", false);

        assert_eq!(
            result,
            Err("invalid download_video payload: expected ident at line 1 column 2".to_string())
        );
    }

    #[test]
    fn it_should_record_the_thumbnail_filename() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::with_listing(vec![
                "fake-output.jpg".to_string(),
            ])),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.start_download(fixed_timestamp()).mark_downloaded(
                Quality::High,
                "fake-output/fake-output.mp4",
                Some("fake-output/fake-output.jpg".to_string()),
                None,
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_record_no_thumbnail_if_none_written() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::with_listing(Vec::new())),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.start_download(fixed_timestamp()).mark_downloaded(
                Quality::High,
                "fake-output/fake-output.mp4",
                None,
                None,
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_record_the_duration() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::with_duration(223)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.start_download(fixed_timestamp()).mark_downloaded(
                Quality::High,
                "fake-output/fake-output.mp4",
                None,
                Some(223),
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_record_no_duration_if_unknown() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.start_download(fixed_timestamp()).mark_downloaded(
                Quality::High,
                "fake-output/fake-output.mp4",
                None,
                None,
                fixed_timestamp(),
            )]
        );
    }

    #[test]
    fn it_should_record_when_metadata_was_generated() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(output_dir.path().join("fake-output")).unwrap();
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            video_metadata_repository.clone(),
        ));
        let payload = Task::DownloadVideo {
            video_id: video.id.as_str().to_string(),
            quality: "high".to_string(),
            output_dir: output_dir.path().to_string_lossy().to_string(),
        }
        .payload()
        .to_string();

        let result = run(&task, &payload, false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(VideoMetadata::new(
                "My Video",
                "A description",
                "My Channel",
                "My Channel",
                fixed_timestamp(),
                None,
                Vec::new(),
                "yt1",
                None,
                "20231114 My Video",
                fixed_timestamp(),
            ))
        );
    }

    #[test]
    fn it_should_keep_the_metadata_creation_time_when_regenerated() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
        let video_dir = output_dir.path().join("fake-output");
        std::fs::create_dir_all(&video_dir).unwrap();
        let video = my_video();
        video_repository.save(&video).unwrap();
        let earlier = DateTime::<Utc>::from_timestamp(1_600_000_000, 0).unwrap();
        video_metadata_repository
            .save(
                &video.id,
                &VideoMetadata::new(
                    "Stale Title",
                    "Stale plot",
                    "Stale Channel",
                    "Stale Channel",
                    earlier,
                    None,
                    Vec::new(),
                    "yt1",
                    None,
                    "20200913 Stale Title",
                    earlier,
                ),
                &video_dir,
            )
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            video_metadata_repository.clone(),
        ));
        let payload = Task::DownloadVideo {
            video_id: video.id.as_str().to_string(),
            quality: "high".to_string(),
            output_dir: output_dir.path().to_string_lossy().to_string(),
        }
        .payload()
        .to_string();

        let result = run(&task, &payload, false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(VideoMetadata {
                created_at: earlier,
                ..VideoMetadata::new(
                    "My Video",
                    "A description",
                    "My Channel",
                    "My Channel",
                    fixed_timestamp(),
                    None,
                    Vec::new(),
                    "yt1",
                    None,
                    "20231114 My Video",
                    fixed_timestamp(),
                )
            })
        );
    }

    #[test]
    fn it_should_save_youtube_metadata() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(output_dir.path().join("fake-output")).unwrap();
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            video_metadata_repository.clone(),
        ));
        let payload = Task::DownloadVideo {
            video_id: video.id.as_str().to_string(),
            quality: "high".to_string(),
            output_dir: output_dir.path().to_string_lossy().to_string(),
        }
        .payload()
        .to_string();

        let result = run(&task, &payload, false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "fake-output/fake-output.mp4",
                        None,
                        None,
                        fixed_timestamp(),
                    )
            ]
        );
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(VideoMetadata::new(
                "My Video",
                "A description",
                "My Channel",
                "My Channel",
                fixed_timestamp(),
                None,
                Vec::new(),
                "yt1",
                None,
                "20231114 My Video",
                fixed_timestamp(),
            ))
        );
    }

    #[test]
    fn it_should_download_even_if_metadata_fetch_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(output_dir.path().join(FAKE_FRESH_FOLDER)).unwrap();
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            video_metadata_repository.clone(),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "fake-output/fake-output.mp4",
                        None,
                        None,
                        fixed_timestamp(),
                    )
            ]
        );
        assert_eq!(video_metadata_repository.find(&video.id).unwrap(), None);
        assert_eq!(
            folder_entries(&output_dir.path().join(FAKE_FRESH_FOLDER)),
            Vec::<String>::new()
        );
    }

    #[test]
    fn it_should_write_movie_nfo_before_the_video_lands() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
        let video_dir = output_dir.path().join(FAKE_FRESH_FOLDER);
        std::fs::create_dir_all(&video_dir).unwrap();
        let video = my_video();
        video_repository.save(&video).unwrap();
        let nfo_while_downloading = Arc::new(std::sync::Mutex::new(None));
        let observed = nfo_while_downloading.clone();
        let observed_dir = video_dir.clone();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::new(true).with_on_download(move || {
                    *observed.lock().unwrap() =
                        std::fs::read_to_string(observed_dir.join(MOVIE_NFO_FILENAME)).ok();
                }),
            ),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            *nfo_while_downloading.lock().unwrap(),
            Some(render_movie_nfo(&generated_metadata(Some(
                "fake-output.jpg"
            ))))
        );
    }

    #[test]
    fn it_should_drop_the_thumbnail_from_movie_nfo_if_none_downloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
        let video_dir = output_dir.path().join(FAKE_FRESH_FOLDER);
        std::fs::create_dir_all(&video_dir).unwrap();
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            std::fs::read_to_string(video_dir.join(MOVIE_NFO_FILENAME)).unwrap(),
            render_movie_nfo(&generated_metadata(None))
        );
    }

    #[test]
    fn it_should_generate_metadata_if_fetch_fails_only_before_the_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
        let video_dir = output_dir.path().join(FAKE_FRESH_FOLDER);
        std::fs::create_dir_all(&video_dir).unwrap();
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FailingOnceYoutubeMetadataRepository::new(youtube_metadata())),
            video_metadata_repository.clone(),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(generated_metadata(None))
        );
        assert_eq!(
            std::fs::read_to_string(video_dir.join(MOVIE_NFO_FILENAME)).unwrap(),
            render_movie_nfo(&generated_metadata(None))
        );
    }

    #[test]
    fn it_should_remove_movie_nfo_if_download_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
        let video_dir = output_dir.path().join("My Video");
        std::fs::create_dir_all(&video_dir).unwrap();
        let video = my_video().with_thumbnail("My Video/My Video.jpg", fixed_timestamp());
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(false)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            video_metadata_repository.clone(),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(
            result,
            Err(format!("yt-dlp failed to download video {}", video.id))
        );
        assert_eq!(video_metadata_repository.find(&video.id).unwrap(), None);
        assert_eq!(folder_entries(&video_dir), Vec::<String>::new());
    }

    #[test]
    #[cfg(unix)]
    fn it_should_detect_a_real_thumbnail_file_written_by_yt_dlp_alongside_the_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let output_dir = unique_temp_dir("download-video-task-thumbnail-e2e");
        let fake_yt_dlp =
            FakeYtDlp::with_downloaded_files("My Video.mp4", &["My Video.mp4", "My Video.jpg"]);
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(YtDlpVideoDownloaderRepository::new(
                fake_yt_dlp.path.clone(),
            )),
            Arc::new(FilesystemVideoFileRepository),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));
        let payload = Task::DownloadVideo {
            video_id: video.id.as_str().to_string(),
            quality: "high".to_string(),
            output_dir: output_dir.to_string_lossy().to_string(),
        }
        .payload()
        .to_string();

        let result = run(&task, &payload, false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.start_download(fixed_timestamp()).mark_downloaded(
                Quality::High,
                "My Video/My Video.mp4",
                Some("My Video/My Video.jpg".to_string()),
                None,
                fixed_timestamp(),
            )]
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }

    /// Builds a downloader around the ports a test seeds, configures or
    /// asserts; the playlist membership lookup (only used to pick a metadata
    /// sorttitle) and the clock are ones no test here varies.
    fn video_downloader(
        db: &TestDatabase,
        video_repository: Arc<SqliteVideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        youtube_metadata_repository: Arc<dyn YoutubeMetadataRepository>,
        video_metadata_repository: Arc<SqliteVideoMetadataRepository>,
    ) -> VideoDownloader {
        VideoDownloader::new(
            video_repository,
            video_downloader_repository,
            video_file_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            youtube_metadata_repository,
            video_metadata_repository,
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    /// A task for tests whose payload is rejected before reaching the
    /// downloader. Its repositories sit on an unmigrated in-memory database,
    /// so a payload that wrongly got through would fail loudly instead of
    /// passing.
    fn any_task() -> DownloadVideoTask {
        DownloadVideoTask::new(VideoDownloader::new(
            Arc::new(SqliteVideoRepository::new(unused_connection())),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqlitePlaylistVideoRepository::new(unused_connection())),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(unused_connection())),
            Arc::new(SqliteEventPublisher::new(
                unused_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
        ))
    }

    fn unused_connection() -> crate::infrastructure::shared::sqlite_connection::Database {
        crate::infrastructure::shared::sqlite_connection::Database::single(
            Connection::open_in_memory().unwrap(),
        )
    }

    fn my_video() -> Video {
        Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp())
    }

    fn youtube_metadata() -> YoutubeMetadata {
        YoutubeMetadata {
            title: "My Video".to_string(),
            description: "A description".to_string(),
            channel_title: "My Channel".to_string(),
            published_at: fixed_timestamp(),
            tags: Vec::new(),
            category_id: None,
        }
    }

    /// One call as `FakeVideoDownloaderRepository` records it, for the
    /// `yt1` video at high quality.
    fn download_call(
        desired_filename: &str,
        output_dir: &str,
        existing_folder: Option<&str>,
    ) -> (String, String, String, Quality, PathBuf, Option<String>) {
        (
            "https://www.youtube.com/watch?v=yt1".to_string(),
            desired_filename.to_string(),
            "yt1".to_string(),
            Quality::High,
            PathBuf::from(output_dir),
            existing_folder.map(str::to_string),
        )
    }

    /// The metadata a download of `my_video()` generates from
    /// `youtube_metadata()`, referencing `thumb` when given.
    fn generated_metadata(thumb: Option<&str>) -> VideoMetadata {
        VideoMetadata::new(
            "My Video",
            "A description",
            "My Channel",
            "My Channel",
            fixed_timestamp(),
            None,
            Vec::new(),
            "yt1",
            thumb.map(str::to_string),
            "20231114 My Video",
            fixed_timestamp(),
        )
    }

    fn output_dir_payload(video: &Video, output_dir: &std::path::Path) -> String {
        Task::DownloadVideo {
            video_id: video.id.as_str().to_string(),
            quality: "high".to_string(),
            output_dir: output_dir.to_string_lossy().to_string(),
        }
        .payload()
        .to_string()
    }

    fn folder_entries(dir: &std::path::Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect()
    }

    fn pending_event(id: i64, event: DomainEvent) -> ScheduledEvent {
        ScheduledEvent {
            id,
            event_type: event.event_type().to_string(),
            payload: event.payload().to_string(),
            retries: 0,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
    }

    fn payload_for(video_id: &str) -> String {
        Task::DownloadVideo {
            video_id: video_id.to_string(),
            quality: "high".to_string(),
            output_dir: "/videos/my-playlist".to_string(),
        }
        .payload()
        .to_string()
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn run(task: &DownloadVideoTask, payload: &str, is_last_attempt: bool) -> Result<(), String> {
        task.handle(payload, is_last_attempt)
            .map_err(|e| e.to_string())
    }
}
