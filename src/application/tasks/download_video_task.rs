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
    use crate::domain::channel::ChannelHandle;
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::services::MetadataGenerator;
    use crate::domain::video::EpisodeNumber;
    use crate::domain::video::PlaybackPosition;
    use crate::domain::video::PlaylistEpisodeNumber;
    use crate::domain::video::Video;
    use crate::domain::video::VideoDownloaded;
    use crate::domain::video::VideoId;
    use crate::domain::video_metadata::{VideoMetadata, render_episode_nfo};
    use crate::infrastructure::repositories::filesystem_video_file_repository::{
        FakeVideoFileRepository, FilesystemVideoFileRepository, VideoFileRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, SqliteChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::{
        SqliteVideoMetadataRepository, VideoMetadataRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::{
        FailingOnceYoutubeMetadataRepository, FakeYoutubeMetadataRepository, YoutubeMetadata,
        YoutubeMetadataRepository,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::{
        DownloadedVideo, FakeVideoDownloaderRepository, VideoDownloaderRepository,
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
    fn it_should_save_a_downloaded_video_as_an_episode_in_its_season_folder() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let output_dir = tempfile::tempdir().unwrap();
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
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
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
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(published_episode(), fixed_timestamp())
            ]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str())),
                    output_dir
                        .path()
                        .join("Season 2026/S2026E031501 - My Video.mp4"),
                ),
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.jpg", video.id.as_str())),
                    output_dir
                        .path()
                        .join("Season 2026/S2026E031501 - My Video.jpg"),
                ),
            ]
        );
        assert_eq!(
            std::fs::read_to_string(
                output_dir
                    .path()
                    .join("Season 2026/S2026E031501 - My Video.nfo")
            )
            .unwrap(),
            render_episode_nfo(
                &generated_metadata(Some("S2026E031501 - My Video.jpg")),
                &published_episode()
            )
        );
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from(format!(
                "/videos/.downloads/{}",
                video.id.as_str()
            ))]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoDownloaded(VideoDownloaded {
                    video_id: video.id.as_str().to_string(),
                    output_dir: output_dir.path().to_string_lossy().to_string(),
                    folder: "Season 2026".to_string(),
                })
            )]
        );
    }

    #[test]
    fn it_should_write_the_episode_nfo_before_moving_the_media_into_place() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
        let nfo_path = output_dir
            .path()
            .join("Season 2026/S2026E031501 - My Video.nfo");
        let nfo_when_media_moved = Arc::new(std::sync::Mutex::new(None));
        let observed = nfo_when_media_moved.clone();
        let observed_nfo_path = nfo_path.clone();
        let video_file_repository = Arc::new(FakeVideoFileRepository::default().with_on_move(
            move |_, to| {
                if to.extension().is_some_and(|ext| ext == "mp4") {
                    *observed.lock().unwrap() = std::fs::read_to_string(&observed_nfo_path).ok();
                }
            },
        ));
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
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            video_file_repository,
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            *nfo_when_media_moved.lock().unwrap(),
            Some(render_episode_nfo(
                &generated_metadata(Some("S2026E031501 - My Video.jpg")),
                &published_episode()
            ))
        );
    }

    #[test]
    fn it_should_number_two_same_day_videos_of_one_show_consecutively() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = SqliteChannelVideoRepository::new(db.database());
        let output_dir = tempfile::tempdir().unwrap();
        let first = Video::create(
            VideoId::new("yt0").unwrap(),
            "First Video",
            fixed_timestamp(),
        )
        .mark_downloaded(
            Quality::High,
            "Season 2026/S2026E031501 - First Video.mp4",
            None,
            None,
            fixed_timestamp(),
        )
        .with_episode_number(published_episode(), fixed_timestamp());
        let video = my_video();
        video_repository.save(&first).unwrap();
        video_repository.update(&first).unwrap();
        video_repository.save(&video).unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                my_channel_id(),
                first.id.clone(),
                1,
                fixed_timestamp(),
            ))
            .unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                2,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                first,
                video
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031502 - My Video.mp4",
                        Some("Season 2026/S2026E031502 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(
                        EpisodeNumber::from_columns(2026, 31502).unwrap(),
                        fixed_timestamp()
                    )
            ]
        );
    }

    #[test]
    fn it_should_reuse_the_number_recorded_by_the_thumbnail_fetch() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let output_dir = tempfile::tempdir().unwrap();
        let prefetched_number = EpisodeNumber::from_columns(2026, 31503).unwrap();
        let video = my_video()
            .with_thumbnail("Season 2026/S2026E031503 - My Video.jpg", fixed_timestamp())
            .with_episode_number(prefetched_number, fixed_timestamp());
        video_repository.save(&video).unwrap();
        video_repository.update(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
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
                        "Season 2026/S2026E031503 - My Video.mp4",
                        Some("Season 2026/S2026E031503 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
            ]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str())),
                    output_dir
                        .path()
                        .join("Season 2026/S2026E031503 - My Video.mp4"),
                ),
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.jpg", video.id.as_str())),
                    output_dir
                        .path()
                        .join("Season 2026/S2026E031503 - My Video.jpg"),
                ),
            ]
        );
    }

    #[test]
    fn it_should_save_a_playlist_video_under_its_playlist_episode_number() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let output_dir = tempfile::tempdir().unwrap();
        let video = my_video().with_episode_number(
            PlaylistEpisodeNumber::new(7).unwrap().into(),
            fixed_timestamp(),
        );
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    published_at: Some(
                        DateTime::parse_from_rfc3339("2014-06-01T10:00:00Z")
                            .unwrap()
                            .to_utc(),
                    ),
                    ..staged_download()
                },
            )),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
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
                        "Season 1/S01E007 - My Video.mp4",
                        Some("Season 1/S01E007 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
            ]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str())),
                    output_dir.path().join("Season 1/S01E007 - My Video.mp4"),
                ),
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.jpg", video.id.as_str())),
                    output_dir.path().join("Season 1/S01E007 - My Video.jpg"),
                ),
            ]
        );
    }

    #[test]
    fn it_should_number_from_the_upload_date_when_no_timestamp_is_reported() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
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
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    published_at: Some(
                        DateTime::parse_from_rfc3339("2026-03-16T00:00:00Z")
                            .unwrap()
                            .to_utc(),
                    ),
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031601 - My Video.mp4",
                        Some("Season 2026/S2026E031601 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(
                        EpisodeNumber::from_columns(2026, 31601).unwrap(),
                        fixed_timestamp()
                    )
            ]
        );
    }

    #[test]
    fn it_should_download_under_its_episode_name_without_metadata() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let output_dir = tempfile::tempdir().unwrap();
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
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            video_file_repository.clone(),
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
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(published_episode(), fixed_timestamp())
            ]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str())),
                    output_dir
                        .path()
                        .join("Season 2026/S2026E031501 - My Video.mp4"),
                ),
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.jpg", video.id.as_str())),
                    output_dir
                        .path()
                        .join("Season 2026/S2026E031501 - My Video.jpg"),
                ),
            ]
        );
        assert_eq!(video_metadata_repository.find(&video.id).unwrap(), None);
        assert_eq!(folder_entries(output_dir.path()), Vec::<String>::new());
    }

    #[test]
    fn it_should_discard_the_staging_dir_when_the_download_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let output_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(false)),
            video_file_repository.clone(),
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
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from(format!(
                "/videos/.downloads/{}",
                video.id.as_str()
            ))]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(video_metadata_repository.find(&video.id).unwrap(), None);
        assert_eq!(folder_entries(output_dir.path()), Vec::<String>::new());
    }

    #[test]
    fn it_should_mark_errored_if_the_download_cannot_be_moved_into_place_on_the_last_attempt() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let output_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        let staged_media =
            PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str()));
        let video_file_repository =
            Arc::new(FakeVideoFileRepository::default().with_failing_move(staged_media.clone()));
        video_repository.save(&video).unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), true);

        assert_eq!(
            result,
            Err(format!("permission denied moving {staged_media:?}"))
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_errored(fixed_timestamp())
                    .with_episode_number(published_episode(), fixed_timestamp())
            ]
        );
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from(format!(
                "/videos/.downloads/{}",
                video.id.as_str()
            ))]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
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
        let output_dir = tempfile::tempdir().unwrap();
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
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    sabr_notice: Some("SABR-only streaming experiment".to_string()),
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(published_episode(), fixed_timestamp())
            ]
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
    fn it_should_discard_the_download_when_the_video_was_deleted_meanwhile() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let output_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.save(&video).unwrap();
        let deleting_repository = video_repository.clone();
        let deleted_id = video.id.clone();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::succeeding_with(staged_download()).with_on_download(
                    move || {
                        deleting_repository.delete(&deleted_id).unwrap();
                    },
                ),
            ),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from(format!(
                "/videos/.downloads/{}",
                video.id.as_str()
            ))]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(folder_entries(output_dir.path()), Vec::<String>::new());
    }

    #[test]
    fn it_should_keep_the_watch_state_recorded_while_downloading() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
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
        let watching_repository = video_repository.clone();
        let watched_id = video.id.clone();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::succeeding_with(staged_download()).with_on_download(
                    move || record_watch_state_meanwhile(&watching_repository, &watched_id),
                ),
            ),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![with_watch_state_recorded_meanwhile(
                video
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(published_episode(), fixed_timestamp())
            )]
        );
    }

    #[test]
    fn it_should_keep_the_watch_state_recorded_while_a_failed_download_ran() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let watching_repository = video_repository.clone();
        let watched_id = video.id.clone();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::new(false).with_on_download(move || {
                    record_watch_state_meanwhile(&watching_repository, &watched_id)
                }),
            ),
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
            vec![with_watch_state_recorded_meanwhile(
                video
                    .start_download(fixed_timestamp())
                    .mark_errored_retrying(fixed_timestamp())
            )]
        );
    }

    #[test]
    fn it_should_keep_the_watch_state_recorded_while_downloading_an_excluded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let watching_repository = video_repository.clone();
        let watched_id = video.id.clone();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::new(false)
                    .with_diagnosed_reason("This video has been removed by the uploader.")
                    .with_on_download(move || {
                        record_watch_state_meanwhile(&watching_repository, &watched_id)
                    }),
            ),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![with_watch_state_recorded_meanwhile(
                video
                    .start_download(fixed_timestamp())
                    .mark_excluded(fixed_timestamp())
            )]
        );
    }

    #[test]
    fn it_should_keep_the_watch_state_recorded_while_downloading_if_placing_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        let staged_media =
            PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str()));
        video_repository.save(&video).unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let watching_repository = video_repository.clone();
        let watched_id = video.id.clone();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::succeeding_with(staged_download()).with_on_download(
                    move || record_watch_state_meanwhile(&watching_repository, &watched_id),
                ),
            ),
            Arc::new(FakeVideoFileRepository::default().with_failing_move(staged_media.clone())),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), true);

        assert_eq!(
            result,
            Err(format!("permission denied moving {staged_media:?}"))
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![with_watch_state_recorded_meanwhile(
                video
                    .start_download(fixed_timestamp())
                    .mark_errored(fixed_timestamp())
                    .with_episode_number(published_episode(), fixed_timestamp())
            )]
        );
    }

    #[test]
    fn it_should_use_the_sanitized_title_as_filename() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
        let video = Video::create(
            VideoId::new("yt1").unwrap(),
            "My: Messy / Title?",
            fixed_timestamp(),
        );
        video_repository.save(&video).unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My- Messy - Title-.mp4",
                        Some("Season 2026/S2026E031501 - My- Messy - Title-.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(published_episode(), fixed_timestamp())
            ]
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
    fn it_should_record_no_thumbnail_if_none_written() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let output_dir = tempfile::tempdir().unwrap();
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
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    thumbnail_path: None,
                    ..staged_download()
                },
            )),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
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
                        "Season 2026/S2026E031501 - My Video.mp4",
                        None,
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(published_episode(), fixed_timestamp())
            ]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str())),
                output_dir
                    .path()
                    .join("Season 2026/S2026E031501 - My Video.mp4"),
            )]
        );
    }

    #[test]
    fn it_should_record_the_duration() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
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
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    duration_seconds: Some(223),
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        Some(223),
                        fixed_timestamp(),
                    )
                    .with_episode_number(published_episode(), fixed_timestamp())
            ]
        );
    }

    #[test]
    fn it_should_keep_the_metadata_creation_time_when_regenerated() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
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
                &published_episode(),
                &output_dir
                    .path()
                    .join("Season 2026/S2026E031501 - My Video.nfo"),
            )
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            video_metadata_repository.clone(),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(VideoMetadata {
                created_at: earlier,
                ..generated_metadata(Some("S2026E031501 - My Video.jpg"))
            })
        );
    }

    #[test]
    fn it_should_save_youtube_metadata() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
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
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            video_metadata_repository.clone(),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(generated_metadata(Some("S2026E031501 - My Video.jpg")))
        );
    }

    #[test]
    fn it_should_drop_the_thumbnail_from_the_episode_nfo_if_none_downloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
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
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    thumbnail_path: None,
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            std::fs::read_to_string(
                output_dir
                    .path()
                    .join("Season 2026/S2026E031501 - My Video.nfo")
            )
            .unwrap(),
            render_episode_nfo(&generated_metadata(None), &published_episode())
        );
    }

    #[test]
    fn it_should_generate_metadata_if_fetch_fails_only_before_the_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let output_dir = tempfile::tempdir().unwrap();
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
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FailingOnceYoutubeMetadataRepository::new(youtube_metadata())),
            video_metadata_repository.clone(),
        ));

        let result = run(&task, &output_dir_payload(&video, output_dir.path()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(generated_metadata(Some("S2026E031501 - My Video.jpg")))
        );
        assert_eq!(
            std::fs::read_to_string(
                output_dir
                    .path()
                    .join("Season 2026/S2026E031501 - My Video.nfo")
            )
            .unwrap(),
            render_episode_nfo(
                &generated_metadata(Some("S2026E031501 - My Video.jpg")),
                &published_episode()
            )
        );
    }

    #[test]
    #[cfg(unix)]
    fn it_should_detect_a_real_thumbnail_file_written_by_yt_dlp_alongside_the_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_root = unique_temp_dir("download-video-task-thumbnail-e2e");
        let output_dir = videos_root.join("playlists/my-playlist");
        let fake_yt_dlp = FakeYtDlp::with_downloaded_files("yt1.mp4", &["yt1.mp4", "yt1.jpg"]);
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
        let task = DownloadVideoTask::new(VideoDownloader::new(
            video_repository.clone(),
            Arc::new(YtDlpVideoDownloaderRepository::new(
                fake_yt_dlp.path.clone(),
            )),
            Arc::new(FilesystemVideoFileRepository),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            videos_root.to_string_lossy(),
        ));

        let result = run(&task, &output_dir_payload(&video, &output_dir), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2023/S2023E111401 - My Video.mp4",
                        Some("Season 2023/S2023E111401 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(
                        EpisodeNumber::from_columns(2023, 111401).unwrap(),
                        fixed_timestamp()
                    )
            ]
        );
        std::fs::remove_dir_all(&videos_root).unwrap();
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
            Arc::new(MetadataGenerator::new(
                youtube_metadata_repository,
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            video_metadata_repository,
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
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
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteVideoMetadataRepository::new(unused_connection())),
            Arc::new(SqliteEventPublisher::new(
                unused_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        ))
    }

    fn unused_connection() -> crate::infrastructure::shared::sqlite_connection::Database {
        crate::infrastructure::shared::sqlite_connection::Database::single(
            Connection::open_in_memory().unwrap(),
        )
    }

    fn my_channel_id() -> ChannelHandle {
        ChannelHandle::new("@somechannel").unwrap()
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

    /// A download `yt-dlp` staged for `my_video()`, published on
    /// 2026-03-15, with its converted thumbnail.
    fn staged_download() -> DownloadedVideo {
        DownloadedVideo {
            media_path: PathBuf::from("yt1.mp4"),
            thumbnail_path: Some(PathBuf::from("yt1.jpg")),
            duration_seconds: None,
            published_at: Some(
                DateTime::parse_from_rfc3339("2026-03-15T10:00:00Z")
                    .unwrap()
                    .to_utc(),
            ),
            sabr_notice: None,
        }
    }

    /// The first episode of `staged_download()`'s publish day.
    fn published_episode() -> EpisodeNumber {
        EpisodeNumber::from_columns(2026, 31501).unwrap()
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

    /// Stores the video as played and watched, the way marking another copy
    /// watched or playing it does while its download runs.
    fn record_watch_state_meanwhile(video_repository: &SqliteVideoRepository, id: &VideoRecordId) {
        let current = video_repository.find(id).unwrap().unwrap();
        video_repository
            .update(&with_watch_state_recorded_meanwhile(current))
            .unwrap();
    }

    fn with_watch_state_recorded_meanwhile(video: Video) -> Video {
        Video {
            watched_at: Some(watched_timestamp()),
            playback_position: PlaybackPosition::new(42).unwrap(),
            last_played_at: Some(watched_timestamp()),
            ..video
        }
    }

    fn watched_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_100, 0).unwrap()
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn run(task: &DownloadVideoTask, payload: &str, is_last_attempt: bool) -> Result<(), String> {
        task.handle(payload, is_last_attempt)
            .map_err(|e| e.to_string())
    }
}
