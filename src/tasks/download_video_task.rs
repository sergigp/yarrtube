use crate::domain::shared::{PlaylistId, Quality, VideoId};
use crate::domain::task::Task;
use crate::domain::video::VideoService;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Downloads one video via `yt-dlp`, scheduled by
/// `subscribers::download_video_on_video_added` whenever a new video shows
/// up in a tracked playlist.
pub struct DownloadVideoTask {
    video_service: VideoService,
}

impl DownloadVideoTask {
    pub fn new(video_service: VideoService) -> Self {
        Self { video_service }
    }
}

impl TaskHandler for DownloadVideoTask {
    fn handle(&self, payload: &str, is_last_attempt: bool) -> anyhow::Result<()> {
        let (playlist_id, video_id, quality) = Task::decode_download_video_payload(payload)?;
        let Ok(playlist_id) = PlaylistId::new(playlist_id) else {
            return Ok(());
        };
        let Ok(video_id) = VideoId::new(video_id) else {
            return Ok(());
        };
        let Ok(quality) = Quality::new(quality) else {
            return Ok(());
        };
        self.video_service
            .download_video(playlist_id, video_id, quality, is_last_attempt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::video::video_filename::VideoFilename;
    use crate::domain::video::{Video, VideoStatus};
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::repositories::youtube_video_repository::FakeYoutubeVideoRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn playlist_id() -> PlaylistId {
        PlaylistId::new("PL1").unwrap()
    }

    fn video_id() -> VideoId {
        VideoId::new("vid1").unwrap()
    }

    fn payload_for(playlist_id: &str, video_id: &str) -> String {
        Task::DownloadVideo {
            playlist_id: playlist_id.to_string(),
            video_id: video_id.to_string(),
            quality: "high".to_string(),
        }
        .payload()
        .to_string()
    }

    fn handler_with(
        seed_playlist: bool,
        seed_video: bool,
        downloader: FakeVideoDownloaderRepository,
    ) -> (DownloadVideoTask, Arc<FakeVideoRepository>) {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        if seed_playlist {
            playlist_repository
                .insert(&Playlist::create(
                    playlist_id(),
                    PlaylistName::new("My Playlist").unwrap(),
                    PlaylistPath::new("my-playlist").unwrap(),
                    Quality::High,
                    PlaylistKind::YoutubeLinked,
                    fixed_timestamp(),
                ))
                .unwrap();
        }
        let video_repository = Arc::new(FakeVideoRepository::default());
        if seed_video {
            video_repository
                .save(&Video::create(
                    playlist_id(),
                    video_id(),
                    "My Video",
                    fixed_timestamp(),
                ))
                .unwrap();
        }

        let video_service = VideoService::new(
            playlist_repository,
            video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeYoutubeVideoRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(downloader),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );

        (DownloadVideoTask::new(video_service), video_repository)
    }

    #[test]
    fn it_should_mark_the_video_downloaded_on_success() {
        let (handler, video_repository) =
            handler_with(true, true, FakeVideoDownloaderRepository::new(true));

        handler.handle(&payload_for("PL1", "vid1"), false).unwrap();

        let video = video_repository
            .find(&playlist_id(), &video_id())
            .unwrap()
            .unwrap();
        assert_eq!(video.status, VideoStatus::Downloaded);
        assert_eq!(video.quality, Some(Quality::High));
        assert_eq!(video.filename, Some("fake-output.mp4".to_string()));
    }

    #[test]
    fn it_should_mark_the_video_errored_retrying_when_the_download_fails_with_retries_left() {
        let (handler, video_repository) =
            handler_with(true, true, FakeVideoDownloaderRepository::new(false));

        let result = handler.handle(&payload_for("PL1", "vid1"), false);

        assert!(result.is_err());
        let video = video_repository
            .find(&playlist_id(), &video_id())
            .unwrap()
            .unwrap();
        assert_eq!(video.status, VideoStatus::ErroredRetrying);
        assert_eq!(video.quality, None);
    }

    #[test]
    fn it_should_mark_the_video_errored_when_the_download_fails_on_the_last_attempt() {
        let (handler, video_repository) =
            handler_with(true, true, FakeVideoDownloaderRepository::new(false));

        let result = handler.handle(&payload_for("PL1", "vid1"), true);

        assert!(result.is_err());
        let video = video_repository
            .find(&playlist_id(), &video_id())
            .unwrap()
            .unwrap();
        assert_eq!(video.status, VideoStatus::Errored);
        assert_eq!(video.quality, None);
    }

    #[test]
    fn it_should_no_op_when_the_playlist_no_longer_exists() {
        let (handler, video_repository) =
            handler_with(false, true, FakeVideoDownloaderRepository::new(true));

        handler.handle(&payload_for("PL1", "vid1"), false).unwrap();

        let video = video_repository
            .find(&playlist_id(), &video_id())
            .unwrap()
            .unwrap();
        assert_eq!(video.status, VideoStatus::Pending);
    }

    #[test]
    fn it_should_no_op_when_the_video_no_longer_exists() {
        let (handler, _video_repository) =
            handler_with(true, false, FakeVideoDownloaderRepository::new(true));

        let result = handler.handle(&payload_for("PL1", "vid1"), false);

        assert!(result.is_ok());
    }

    #[test]
    fn it_should_pass_the_sanitized_title_as_the_desired_filename_to_the_downloader() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert(&Playlist::create(
                playlist_id(),
                PlaylistName::new("My Playlist").unwrap(),
                PlaylistPath::new("my-playlist").unwrap(),
                Quality::High,
                PlaylistKind::YoutubeLinked,
                fixed_timestamp(),
            ))
            .unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        let messy_title = "My: Messy / Title?";
        video_repository
            .save(&Video::create(
                playlist_id(),
                video_id(),
                messy_title,
                fixed_timestamp(),
            ))
            .unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));

        let video_service = VideoService::new(
            playlist_repository,
            video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeYoutubeVideoRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FakeTaskRepository::default()),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let handler = DownloadVideoTask::new(video_service);

        handler.handle(&payload_for("PL1", "vid1"), false).unwrap();

        let calls = downloader.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let (_, desired_filename, id, _, _) = &calls[0];
        assert_eq!(
            desired_filename,
            VideoFilename::from_title(messy_title).as_str()
        );
        assert_ne!(desired_filename, messy_title);
        assert_eq!(id, video_id().as_str());
    }

    #[test]
    fn it_should_build_a_nested_output_dir_from_a_multi_segment_playlist_path() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert(&Playlist::create(
                playlist_id(),
                PlaylistName::new("My Playlist").unwrap(),
                PlaylistPath::new("a/b/c").unwrap(),
                Quality::High,
                PlaylistKind::YoutubeLinked,
                fixed_timestamp(),
            ))
            .unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository
            .save(&Video::create(
                playlist_id(),
                video_id(),
                "My Video",
                fixed_timestamp(),
            ))
            .unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));

        let video_service = VideoService::new(
            playlist_repository,
            video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeYoutubeVideoRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FakeTaskRepository::default()),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let handler = DownloadVideoTask::new(video_service);

        handler.handle(&payload_for("PL1", "vid1"), false).unwrap();

        let calls = downloader.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let (_, _, _, _, output_dir) = &calls[0];
        assert_eq!(output_dir, std::path::Path::new("/videos/a/b/c"));
    }

    #[test]
    fn it_should_no_op_when_the_payload_ids_are_invalid() {
        let (handler, video_repository) =
            handler_with(true, true, FakeVideoDownloaderRepository::new(true));

        handler.handle(&payload_for("", "vid1"), false).unwrap();

        let video = video_repository
            .find(&playlist_id(), &video_id())
            .unwrap()
            .unwrap();
        assert_eq!(video.status, VideoStatus::Pending);
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let (handler, _video_repository) =
            handler_with(true, true, FakeVideoDownloaderRepository::new(true));

        assert!(handler.handle("not json", false).is_err());
    }
}
