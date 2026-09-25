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
    use crate::domain::video::Video;
    use crate::domain::video::VideoId;
    use crate::domain::video_metadata::VideoMetadata;
    use crate::infrastructure::repositories::filesystem_video_file_repository::{
        FakeVideoFileRepository, FilesystemVideoFileRepository, VideoFileRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::{
        SqliteVideoMetadataRepository, VideoMetadataRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::{
        FakeYoutubeMetadataRepository, YoutubeMetadata,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::{
        FakeVideoDownloaderRepository, VideoDownloaderRepository, YtDlpVideoDownloaderRepository,
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
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
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
    fn it_should_retry_a_failed_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(false)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
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
    fn it_should_record_the_download_error() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
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
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
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
    fn it_should_mark_errored_after_last_attempt() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(false)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
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
    fn it_should_skip_if_video_is_gone() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
        ));

        let result = run(&task, &payload_for(my_video().id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(*downloader.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_use_the_sanitized_title_as_filename() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
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
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            *downloader.calls.lock().unwrap(),
            vec![download_call(
                "My- Messy - Title-",
                "/videos/my-playlist",
                None
            )]
        );
    }

    #[test]
    fn it_should_download_into_the_payload_output_dir() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
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
            vec![download_call("My Video", "/videos/a/b/c", None)]
        );
    }

    #[test]
    fn it_should_reuse_the_existing_video_folder() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = my_video().with_thumbnail("My Video/My Video.jpg", fixed_timestamp());
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
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
    fn it_should_use_no_existing_folder_without_thumbnail() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
        ));

        let result = run(&task, &payload_for(video.id.as_str()), false);

        assert_eq!(result, Ok(()));
        assert_eq!(
            *downloader.calls.lock().unwrap(),
            vec![download_call("My Video", "/videos/my-playlist", None)]
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
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
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
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
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
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::with_listing(Vec::new())),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
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
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::with_duration(223)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
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
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
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
    fn it_should_save_youtube_metadata() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video_metadata_repository =
            Arc::new(SqliteVideoMetadataRepository::new(db.connection()));
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
                "2023-11-14",
                2023,
                None,
                Vec::new(),
                "yt1",
                None,
                "20231114 My Video",
            ))
        );
    }

    #[test]
    fn it_should_download_even_if_metadata_fetch_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video_metadata_repository =
            Arc::new(SqliteVideoMetadataRepository::new(db.connection()));
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

        let result = run(&task, &payload_for(video.id.as_str()), false);

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
    }

    #[test]
    #[cfg(unix)]
    fn it_should_detect_a_real_thumbnail_file_written_by_yt_dlp_alongside_the_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
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
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
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
        youtube_metadata_repository: Arc<FakeYoutubeMetadataRepository>,
        video_metadata_repository: Arc<SqliteVideoMetadataRepository>,
    ) -> VideoDownloader {
        VideoDownloader::new(
            video_repository,
            video_downloader_repository,
            video_file_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection())),
            youtube_metadata_repository,
            video_metadata_repository,
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
            Arc::new(FixedClock(fixed_timestamp())),
        ))
    }

    fn unused_connection() -> Connection {
        Connection::open_in_memory().unwrap()
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
