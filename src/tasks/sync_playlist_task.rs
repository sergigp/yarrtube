use crate::domain::shared::PlaylistId;
use crate::domain::task::Task;
use crate::domain::video::VideoService;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Runs every subsequent sync for a playlist (the first one is triggered by
/// `subscribers::sync_playlist_on_playlist_created` instead).
pub struct SyncPlaylistTask {
    video_service: VideoService,
}

impl SyncPlaylistTask {
    pub fn new(video_service: VideoService) -> Self {
        Self { video_service }
    }
}

impl TaskHandler for SyncPlaylistTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let playlist_id = Task::decode_sync_playlist_payload(payload)?;
        let Ok(playlist_id) = PlaylistId::new(playlist_id) else {
            return Ok(());
        };
        self.video_service.sync_playlist_videos(playlist_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::DomainEvent;
    use crate::domain::playlist::{Playlist, PlaylistName, Quality};
    use crate::domain::shared::VideoId;
    use crate::domain::video::{Video, VideoStatus};
    use crate::infrastructure::repositories::sqlite_event_repository::FakeEventPublisher;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::system_clock::FixedClock;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::{
        FakeYoutubePlaylistItemsRepository, PlaylistVideo,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[allow(clippy::type_complexity)]
    fn handler_with_playlist(
        current_videos: Vec<PlaylistVideo>,
    ) -> (
        SyncPlaylistTask,
        Arc<FakeEventPublisher>,
        Arc<FakeVideoRepository>,
        Arc<FakeTaskRepository>,
    ) {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert_with_event(
                &Playlist::create(
                    PlaylistId::new("PL1").unwrap(),
                    PlaylistName::new("My Playlist").unwrap(),
                    Quality::High,
                    fixed_timestamp(),
                ),
                &DomainEvent::PlaylistCreated {
                    playlist_id: "PL1".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_repository = Arc::new(FakeTaskRepository::default());

        let video_service = VideoService::new(
            playlist_repository,
            video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository {
                videos: current_videos,
            }),
            event_publisher.clone(),
            task_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );

        (
            SyncPlaylistTask::new(video_service),
            event_publisher,
            video_repository,
            task_repository,
        )
    }

    fn payload_for(playlist_id: &str) -> String {
        Task::SyncPlaylist {
            playlist_id: playlist_id.to_string(),
        }
        .payload()
        .to_string()
    }

    #[test]
    fn it_should_no_op_when_the_playlist_no_longer_exists() {
        let (handler, event_publisher, video_repository, task_repository) =
            handler_with_playlist(Vec::new());

        handler.handle(&payload_for("PL404"), false).unwrap();

        assert!(event_publisher.published.lock().unwrap().is_empty());
        assert!(video_repository.videos.lock().unwrap().is_empty());
        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_no_op_when_the_payload_playlist_id_is_invalid() {
        let (handler, event_publisher, video_repository, task_repository) =
            handler_with_playlist(Vec::new());

        handler.handle(&payload_for(""), false).unwrap();

        assert!(event_publisher.published.lock().unwrap().is_empty());
        assert!(video_repository.videos.lock().unwrap().is_empty());
        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_persist_new_videos_and_publish_an_event_per_video() {
        let (handler, event_publisher, video_repository, _tasks) =
            handler_with_playlist(vec![PlaylistVideo {
                video_id: "vid1".to_string(),
                title: "One".to_string(),
            }]);

        handler.handle(&payload_for("PL1"), false).unwrap();

        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].status, VideoStatus::Pending);
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::VideoAdded {
                playlist_id: "PL1".to_string(),
                video_id: "vid1".to_string(),
            }]
        );
    }

    #[test]
    fn it_should_delete_videos_no_longer_present_on_youtube() {
        let (handler, _events, video_repository, _tasks) = handler_with_playlist(Vec::new());
        video_repository.videos.lock().unwrap().push(Video::create(
            PlaylistId::new("PL1").unwrap(),
            VideoId::new("vid1").unwrap(),
            "Stale",
            fixed_timestamp(),
        ));

        handler.handle(&payload_for("PL1"), false).unwrap();

        assert!(video_repository.videos.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_always_schedule_the_next_sync_even_with_no_changes() {
        let (handler, _events, _videos, task_repository) = handler_with_playlist(Vec::new());

        handler.handle(&payload_for("PL1"), false).unwrap();

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert_eq!(scheduled.len(), 1);
        assert_eq!(
            scheduled[0].0,
            Task::SyncPlaylist {
                playlist_id: "PL1".to_string()
            }
        );
        assert_eq!(
            scheduled[0].1,
            fixed_timestamp() + chrono::Duration::seconds(3600)
        );
    }
}
