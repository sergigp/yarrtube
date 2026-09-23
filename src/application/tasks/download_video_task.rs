use crate::domain::services::VideoDownloader;
use crate::domain::shared::{Quality, VideoRecordId};
use crate::domain::task::Task;
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
    use crate::domain::video::video_filename::VideoFilename;
    use crate::domain::video::{Video, VideoStatus};

    use crate::domain::shared::VideoId;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };

    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;

    use crate::infrastructure::repositories::sqlite_playlist_video_repository::FakePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::FakeVideoMetadataRepository;
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    fn fake_metadata_deps(
        video_repository: &Arc<FakeVideoRepository>,
    ) -> (
        Arc<FakePlaylistVideoRepository>,
        Arc<FakeYoutubeMetadataRepository>,
        Arc<FakeVideoMetadataRepository>,
    ) {
        (
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
        )
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
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

    fn handler_with(
        seed_video: bool,
        downloader: FakeVideoDownloaderRepository,
    ) -> (DownloadVideoTask, Arc<FakeVideoRepository>, Video) {
        handler_with_files(seed_video, downloader, FakeVideoFileRepository::default()).0
    }

    fn handler_with_files(
        seed_video: bool,
        downloader: FakeVideoDownloaderRepository,
        video_file_repository: FakeVideoFileRepository,
    ) -> (
        (DownloadVideoTask, Arc<FakeVideoRepository>, Video),
        Arc<FakeVideoFileRepository>,
    ) {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp());
        if seed_video {
            video_repository.save(&video).unwrap();
        }
        let video_file_repository = Arc::new(video_file_repository);
        let (playlist_video_repository, youtube_metadata_repository, video_metadata_repository) =
            fake_metadata_deps(&video_repository);

        let video_downloader = VideoDownloader::new(
            video_repository.clone(),
            Arc::new(downloader),
            video_file_repository.clone(),
            playlist_video_repository,
            youtube_metadata_repository,
            video_metadata_repository,
            Arc::new(FixedClock(fixed_timestamp())),
        );

        (
            (
                DownloadVideoTask::new(video_downloader),
                video_repository,
                video,
            ),
            video_file_repository,
        )
    }

    #[test]
    fn it_should_mark_the_video_downloaded_on_success() {
        let (handler, video_repository, video) =
            handler_with(true, FakeVideoDownloaderRepository::new(true));

        handler
            .handle(&payload_for(video.id.as_str()), false)
            .unwrap();

        let found = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::Downloaded);
        assert_eq!(found.quality, Some(Quality::High));
        assert_eq!(
            found.filename,
            Some("fake-output/fake-output.mp4".to_string())
        );
    }

    #[test]
    fn it_should_mark_the_video_errored_retrying_when_the_download_fails_with_retries_left() {
        let (handler, video_repository, video) =
            handler_with(true, FakeVideoDownloaderRepository::new(false));

        let result = handler.handle(&payload_for(video.id.as_str()), false);

        assert!(result.is_err());
        let found = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::ErroredRetrying);
        assert_eq!(found.quality, None);
    }

    #[test]
    fn it_should_record_yt_dlps_actual_error_message_when_a_download_fails() {
        let (handler, _video_repository, video) = handler_with(
            true,
            FakeVideoDownloaderRepository::with_failed_stderr("HTTP Error 403: Forbidden"),
        );

        let result = handler.handle(&payload_for(video.id.as_str()), false);

        assert_eq!(result.unwrap_err().to_string(), "HTTP Error 403: Forbidden");
    }

    #[test]
    fn it_should_mark_the_video_errored_when_the_download_fails_on_the_last_attempt() {
        let (handler, video_repository, video) =
            handler_with(true, FakeVideoDownloaderRepository::new(false));

        let result = handler.handle(&payload_for(video.id.as_str()), true);

        assert!(result.is_err());
        let found = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::Errored);
        assert_eq!(found.quality, None);
    }

    #[test]
    fn it_should_no_op_when_the_video_no_longer_exists() {
        let (handler, _video_repository, video) =
            handler_with(false, FakeVideoDownloaderRepository::new(true));

        let result = handler.handle(&payload_for(video.id.as_str()), false);

        assert!(result.is_ok());
    }

    #[test]
    fn it_should_pass_the_sanitized_title_as_the_desired_filename_to_the_downloader() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let messy_title = "My: Messy / Title?";
        let video = Video::create(VideoId::new("yt1").unwrap(), messy_title, fixed_timestamp());
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let (playlist_video_repository, youtube_metadata_repository, video_metadata_repository) =
            fake_metadata_deps(&video_repository);

        let video_downloader = VideoDownloader::new(
            video_repository,
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            playlist_video_repository,
            youtube_metadata_repository,
            video_metadata_repository,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let handler = DownloadVideoTask::new(video_downloader);

        handler
            .handle(&payload_for(video.id.as_str()), false)
            .unwrap();

        let calls = downloader.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let (_, desired_filename, id, _, _, _) = &calls[0];
        assert_eq!(
            desired_filename,
            VideoFilename::from_title(messy_title).as_str()
        );
        assert_ne!(desired_filename, messy_title);
        assert_eq!(id, "yt1");
    }

    #[test]
    fn it_should_pass_the_output_dir_from_the_payload_straight_through() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp());
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let (playlist_video_repository, youtube_metadata_repository, video_metadata_repository) =
            fake_metadata_deps(&video_repository);

        let video_downloader = VideoDownloader::new(
            video_repository,
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            playlist_video_repository,
            youtube_metadata_repository,
            video_metadata_repository,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let handler = DownloadVideoTask::new(video_downloader);

        let payload = Task::DownloadVideo {
            video_id: video.id.as_str().to_string(),
            quality: "high".to_string(),
            output_dir: "/videos/a/b/c".to_string(),
        }
        .payload()
        .to_string();
        handler.handle(&payload, false).unwrap();

        let calls = downloader.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let (_, _, _, _, output_dir, _) = &calls[0];
        assert_eq!(output_dir, std::path::Path::new("/videos/a/b/c"));
    }

    #[test]
    fn it_should_pass_the_existing_folder_derived_from_the_thumbnail_filename() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp())
            .with_thumbnail("My Video/My Video.jpg", fixed_timestamp());
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let (playlist_video_repository, youtube_metadata_repository, video_metadata_repository) =
            fake_metadata_deps(&video_repository);

        let video_downloader = VideoDownloader::new(
            video_repository,
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            playlist_video_repository,
            youtube_metadata_repository,
            video_metadata_repository,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let handler = DownloadVideoTask::new(video_downloader);

        handler
            .handle(&payload_for(video.id.as_str()), false)
            .unwrap();

        let calls = downloader.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let (_, _, _, _, _, existing_folder) = &calls[0];
        assert_eq!(existing_folder, &Some("My Video".to_string()));
    }

    #[test]
    fn it_should_pass_no_existing_folder_when_the_video_has_no_thumbnail() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp());
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let (playlist_video_repository, youtube_metadata_repository, video_metadata_repository) =
            fake_metadata_deps(&video_repository);

        let video_downloader = VideoDownloader::new(
            video_repository,
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            playlist_video_repository,
            youtube_metadata_repository,
            video_metadata_repository,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let handler = DownloadVideoTask::new(video_downloader);

        handler
            .handle(&payload_for(video.id.as_str()), false)
            .unwrap();

        let calls = downloader.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let (_, _, _, _, _, existing_folder) = &calls[0];
        assert_eq!(existing_folder, &None);
    }

    #[test]
    fn it_should_no_op_when_the_payload_video_id_is_invalid() {
        let (handler, video_repository, video) =
            handler_with(true, FakeVideoDownloaderRepository::new(true));

        handler.handle(&payload_for(""), false).unwrap();

        let found = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::Pending);
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let (handler, _video_repository, _video) =
            handler_with(true, FakeVideoDownloaderRepository::new(true));

        assert!(handler.handle("not json", false).is_err());
    }

    #[test]
    fn it_should_record_the_thumbnail_filename_when_one_was_written() {
        let ((handler, video_repository, video), _files) = handler_with_files(
            true,
            FakeVideoDownloaderRepository::new(true),
            FakeVideoFileRepository::with_listing(vec!["fake-output.jpg".to_string()]),
        );

        handler
            .handle(&payload_for(video.id.as_str()), false)
            .unwrap();

        let found = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(
            found.thumbnail_filename,
            Some("fake-output/fake-output.jpg".to_string())
        );
    }

    #[test]
    fn it_should_record_no_thumbnail_filename_when_none_was_written() {
        let ((handler, video_repository, video), _files) = handler_with_files(
            true,
            FakeVideoDownloaderRepository::new(true),
            FakeVideoFileRepository::with_listing(Vec::new()),
        );

        handler
            .handle(&payload_for(video.id.as_str()), false)
            .unwrap();

        let found = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.thumbnail_filename, None);
    }

    #[test]
    fn it_should_record_the_duration_reported_by_the_downloader() {
        let (handler, video_repository, video) =
            handler_with(true, FakeVideoDownloaderRepository::with_duration(223));

        handler
            .handle(&payload_for(video.id.as_str()), false)
            .unwrap();

        let found = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.duration_seconds, Some(223));
    }

    #[test]
    fn it_should_record_no_duration_when_the_downloader_reports_none() {
        let (handler, video_repository, video) =
            handler_with(true, FakeVideoDownloaderRepository::new(true));

        handler
            .handle(&payload_for(video.id.as_str()), false)
            .unwrap();

        let found = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.duration_seconds, None);
    }

    #[test]
    fn it_should_save_video_metadata_when_the_youtube_metadata_fetch_succeeds() {
        use crate::infrastructure::repositories::youtube_metadata_repository::YoutubeMetadata;

        let video_repository = Arc::new(FakeVideoRepository::default());
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp());
        video_repository.save(&video).unwrap();
        let video_metadata_repository = Arc::new(FakeVideoMetadataRepository::default());
        let youtube_metadata_repository = Arc::new(FakeYoutubeMetadataRepository {
            metadata: Some(YoutubeMetadata {
                title: "My Video".to_string(),
                description: "A description".to_string(),
                channel_title: "My Channel".to_string(),
                published_at: fixed_timestamp(),
                tags: Vec::new(),
                category_id: None,
            }),
        });

        let video_downloader = VideoDownloader::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            youtube_metadata_repository,
            video_metadata_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let handler = DownloadVideoTask::new(video_downloader);

        handler
            .handle(&payload_for(video.id.as_str()), false)
            .unwrap();

        let found = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::Downloaded);
        assert!(
            video_metadata_repository
                .entries
                .lock()
                .unwrap()
                .iter()
                .any(|(id, _)| *id == video.id)
        );
    }

    #[test]
    fn it_should_still_mark_the_video_downloaded_and_save_no_metadata_when_the_youtube_metadata_fetch_fails()
     {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp());
        video_repository.save(&video).unwrap();
        let video_metadata_repository = Arc::new(FakeVideoMetadataRepository::default());

        let video_downloader = VideoDownloader::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            video_metadata_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let handler = DownloadVideoTask::new(video_downloader);

        handler
            .handle(&payload_for(video.id.as_str()), false)
            .unwrap();

        let found = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::Downloaded);
        assert!(video_metadata_repository.entries.lock().unwrap().is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn it_should_detect_a_real_thumbnail_file_written_by_yt_dlp_alongside_the_video() {
        use crate::infrastructure::repositories::filesystem_video_file_repository::FilesystemVideoFileRepository;
        use crate::infrastructure::repositories::youtube_video_downloader_repository::YtDlpVideoDownloaderRepository;
        use crate::infrastructure::shared::ytdlp::test_support::{FakeYtDlp, unique_temp_dir};

        let output_dir = unique_temp_dir("download-video-task-thumbnail-e2e");
        let fake =
            FakeYtDlp::with_downloaded_files("My Video.mp4", &["My Video.mp4", "My Video.jpg"]);
        let video_repository = Arc::new(FakeVideoRepository::default());
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp());
        video_repository.save(&video).unwrap();
        let (playlist_video_repository, youtube_metadata_repository, video_metadata_repository) =
            fake_metadata_deps(&video_repository);

        let video_downloader = VideoDownloader::new(
            video_repository.clone(),
            Arc::new(YtDlpVideoDownloaderRepository::new(fake.path.clone())),
            Arc::new(FilesystemVideoFileRepository),
            playlist_video_repository,
            youtube_metadata_repository,
            video_metadata_repository,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let handler = DownloadVideoTask::new(video_downloader);

        let payload = Task::DownloadVideo {
            video_id: video.id.as_str().to_string(),
            quality: "high".to_string(),
            output_dir: output_dir.to_string_lossy().to_string(),
        }
        .payload()
        .to_string();
        handler.handle(&payload, false).unwrap();

        let found = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.filename, Some("My Video/My Video.mp4".to_string()));
        assert_eq!(
            found.thumbnail_filename,
            Some("My Video/My Video.jpg".to_string())
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }
}
