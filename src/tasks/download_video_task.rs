use crate::domain::playlist::Quality;
use crate::domain::shared::{PlaylistId, VideoId};
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
    use crate::domain::playlist::{Playlist, PlaylistName, Quality};
    use crate::domain::video::{Video, VideoStatus};
    use crate::infrastructure::repositories::sqlite_event_repository::FakeEventPublisher;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::system_clock::FixedClock;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
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

    fn handler_with_seeded_video(succeeds: bool) -> (DownloadVideoTask, Arc<FakeVideoRepository>) {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert(&Playlist::create(
                PlaylistId::new("PL1").unwrap(),
                PlaylistName::new("My Playlist").unwrap(),
                Quality::High,
                fixed_timestamp(),
            ))
            .unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository
            .save(&Video::create(
                PlaylistId::new("PL1").unwrap(),
                VideoId::new("vid1").unwrap(),
                "My Video",
                fixed_timestamp(),
            ))
            .unwrap();

        let video_service = VideoService::new(
            playlist_repository,
            video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeVideoDownloaderRepository::new(succeeds)),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );

        (DownloadVideoTask::new(video_service), video_repository)
    }

    #[test]
    fn it_should_mark_the_video_downloaded_on_success() {
        let (handler, video_repository) = handler_with_seeded_video(true);

        handler.handle(&payload_for("PL1", "vid1"), false).unwrap();

        let video = video_repository
            .find(
                &PlaylistId::new("PL1").unwrap(),
                &VideoId::new("vid1").unwrap(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(video.status, VideoStatus::Downloaded);
    }

    #[test]
    fn it_should_no_op_when_the_payload_ids_are_invalid() {
        let (handler, video_repository) = handler_with_seeded_video(true);

        handler.handle(&payload_for("", "vid1"), false).unwrap();

        let video = video_repository
            .find(
                &PlaylistId::new("PL1").unwrap(),
                &VideoId::new("vid1").unwrap(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(video.status, VideoStatus::Pending);
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let (handler, _video_repository) = handler_with_seeded_video(true);

        assert!(handler.handle("not json", false).is_err());
    }
}
