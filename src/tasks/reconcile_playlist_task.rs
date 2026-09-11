use crate::domain::shared::PlaylistId;
use crate::domain::task::Task;
use crate::domain::video::VideoService;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Runs every subsequent reconcile for a playlist (the first one is
/// triggered by `subscribers::reconcile_on_playlist_created` instead).
pub struct ReconcilePlaylistTask {
    video_service: VideoService,
}

impl ReconcilePlaylistTask {
    pub fn new(video_service: VideoService) -> Self {
        Self { video_service }
    }
}

impl TaskHandler for ReconcilePlaylistTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let playlist_id = Task::decode_reconcile_playlist_payload(payload)?;
        let Ok(playlist_id) = PlaylistId::new(playlist_id) else {
            return Ok(());
        };
        self.video_service.reconcile_playlist(playlist_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::DomainEvent;
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::shared::{Quality, VideoId};
    use crate::domain::video::{Video, VideoStatus};
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::{
        FakeYoutubePlaylistItemsRepository, PlaylistVideo,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::repositories::youtube_video_repository::FakeYoutubeVideoRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[allow(clippy::type_complexity)]
    fn handler_with(
        kind: PlaylistKind,
        current_videos: Vec<PlaylistVideo>,
        video_file_repository: FakeVideoFileRepository,
    ) -> (
        ReconcilePlaylistTask,
        Arc<FakeEventPublisher>,
        Arc<FakeVideoRepository>,
        Arc<FakeTaskRepository>,
        Arc<FakeVideoFileRepository>,
    ) {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert(&Playlist::create(
                PlaylistId::new("PL1").unwrap(),
                PlaylistName::new("My Playlist").unwrap(),
                PlaylistPath::new("my-playlist").unwrap(),
                Quality::High,
                kind,
                fixed_timestamp(),
            ))
            .unwrap();
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_repository = Arc::new(FakeTaskRepository::default());
        let video_file_repository = Arc::new(video_file_repository);

        let video_service = VideoService::new(
            playlist_repository,
            video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository {
                videos: current_videos,
            }),
            Arc::new(FakeYoutubeVideoRepository::default()),
            event_publisher.clone(),
            task_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            video_file_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );

        (
            ReconcilePlaylistTask::new(video_service),
            event_publisher,
            video_repository,
            task_repository,
            video_file_repository,
        )
    }

    fn payload_for(playlist_id: &str) -> String {
        Task::ReconcilePlaylist {
            playlist_id: playlist_id.to_string(),
        }
        .payload()
        .to_string()
    }

    #[test]
    fn it_should_no_op_when_the_playlist_no_longer_exists() {
        let (handler, event_publisher, video_repository, task_repository, _files) = handler_with(
            PlaylistKind::YoutubeLinked,
            Vec::new(),
            FakeVideoFileRepository::default(),
        );

        handler.handle(&payload_for("PL404"), false).unwrap();

        assert!(event_publisher.published.lock().unwrap().is_empty());
        assert!(video_repository.videos.lock().unwrap().is_empty());
        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_no_op_when_the_payload_playlist_id_is_invalid() {
        let (handler, event_publisher, video_repository, task_repository, _files) = handler_with(
            PlaylistKind::YoutubeLinked,
            Vec::new(),
            FakeVideoFileRepository::default(),
        );

        handler.handle(&payload_for(""), false).unwrap();

        assert!(event_publisher.published.lock().unwrap().is_empty());
        assert!(video_repository.videos.lock().unwrap().is_empty());
        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_persist_new_videos_and_publish_an_event_per_video_for_a_youtube_linked_playlist() {
        let (handler, event_publisher, video_repository, _tasks, _files) = handler_with(
            PlaylistKind::YoutubeLinked,
            vec![PlaylistVideo {
                video_id: "vid1".to_string(),
                title: "One".to_string(),
            }],
            FakeVideoFileRepository::default(),
        );

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
    fn it_should_leave_an_existing_videos_status_unchanged_while_refreshing_its_title() {
        let (handler, event_publisher, video_repository, _tasks, _files) = handler_with(
            PlaylistKind::YoutubeLinked,
            vec![PlaylistVideo {
                video_id: "vid1".to_string(),
                title: "Renamed".to_string(),
            }],
            FakeVideoFileRepository::default(),
        );
        video_repository.videos.lock().unwrap().push(
            Video::create(
                PlaylistId::new("PL1").unwrap(),
                VideoId::new("vid1").unwrap(),
                "Original",
                fixed_timestamp(),
            )
            .start_download(fixed_timestamp()),
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].title, "Renamed");
        assert_eq!(stored[0].status, VideoStatus::InProgress);
        assert!(event_publisher.published.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_delete_videos_no_longer_present_on_youtube() {
        let (handler, _events, video_repository, _tasks, _files) = handler_with(
            PlaylistKind::YoutubeLinked,
            Vec::new(),
            FakeVideoFileRepository::default(),
        );
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
    fn it_should_publish_a_video_deleted_event_with_the_correct_was_downloaded_per_video() {
        let (handler, event_publisher, video_repository, _tasks, _files) = handler_with(
            PlaylistKind::YoutubeLinked,
            Vec::new(),
            FakeVideoFileRepository::default(),
        );
        video_repository.videos.lock().unwrap().push(
            Video::create(
                PlaylistId::new("PL1").unwrap(),
                VideoId::new("vid1").unwrap(),
                "Downloaded Video",
                fixed_timestamp(),
            )
            .start_download(fixed_timestamp())
            .mark_downloaded(Quality::High, "Downloaded Video.mp4", fixed_timestamp()),
        );
        video_repository.videos.lock().unwrap().push(Video::create(
            PlaylistId::new("PL1").unwrap(),
            VideoId::new("vid2").unwrap(),
            "Pending Video",
            fixed_timestamp(),
        ));

        handler.handle(&payload_for("PL1"), false).unwrap();

        let published = event_publisher.published.lock().unwrap();
        assert_eq!(
            *published,
            vec![
                DomainEvent::VideoDeleted {
                    playlist_id: "PL1".to_string(),
                    video_id: "vid1".to_string(),
                    title: "Downloaded Video".to_string(),
                    filename: Some("Downloaded Video.mp4".to_string()),
                    was_downloaded: true,
                },
                DomainEvent::VideoDeleted {
                    playlist_id: "PL1".to_string(),
                    video_id: "vid2".to_string(),
                    title: "Pending Video".to_string(),
                    filename: None,
                    was_downloaded: false,
                },
            ]
        );
    }

    #[test]
    fn it_should_always_schedule_the_next_reconcile_even_with_no_changes() {
        let (handler, _events, _videos, task_repository, _files) = handler_with(
            PlaylistKind::YoutubeLinked,
            Vec::new(),
            FakeVideoFileRepository::default(),
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert_eq!(scheduled.len(), 1);
        assert_eq!(
            scheduled[0].0,
            Task::ReconcilePlaylist {
                playlist_id: "PL1".to_string()
            }
        );
        assert_eq!(
            scheduled[0].1,
            fixed_timestamp() + chrono::Duration::seconds(3600)
        );
    }

    #[test]
    fn it_should_skip_the_membership_diff_for_a_custom_playlist() {
        let (handler, event_publisher, video_repository, task_repository, _files) = handler_with(
            PlaylistKind::Custom,
            vec![PlaylistVideo {
                video_id: "vid1".to_string(),
                title: "One".to_string(),
            }],
            FakeVideoFileRepository::default(),
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        assert!(video_repository.videos.lock().unwrap().is_empty());
        assert!(event_publisher.published.lock().unwrap().is_empty());
        assert_eq!(task_repository.scheduled.lock().unwrap().len(), 1);
    }

    #[test]
    fn it_should_heal_a_downloaded_video_whose_file_is_missing() {
        let (handler, _events, video_repository, task_repository, _files) = handler_with(
            PlaylistKind::Custom,
            Vec::new(),
            FakeVideoFileRepository::with_listing(Vec::new()),
        );
        video_repository.videos.lock().unwrap().push(
            Video::create(
                PlaylistId::new("PL1").unwrap(),
                VideoId::new("vid1").unwrap(),
                "My Video",
                fixed_timestamp(),
            )
            .start_download(fixed_timestamp())
            .mark_downloaded(Quality::High, "My Video.mp4", fixed_timestamp()),
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored[0].status, VideoStatus::Pending);
        assert_eq!(stored[0].filename, None);
        assert_eq!(stored[0].quality, None);

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert!(scheduled.iter().any(|(task, _)| *task
            == Task::DownloadVideo {
                playlist_id: "PL1".to_string(),
                video_id: "vid1".to_string(),
                quality: "high".to_string(),
            }));
    }

    #[test]
    fn it_should_delete_an_orphaned_file() {
        let (handler, _events, _videos, _tasks, files) = handler_with(
            PlaylistKind::Custom,
            Vec::new(),
            FakeVideoFileRepository::with_listing(vec!["orphan.mp4".to_string()]),
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        let calls = files.deleted_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, "orphan.mp4");
    }

    #[test]
    fn it_should_leave_a_matching_file_alone() {
        let (handler, _events, video_repository, _tasks, files) = handler_with(
            PlaylistKind::Custom,
            Vec::new(),
            FakeVideoFileRepository::with_listing(vec!["My Video.mp4".to_string()]),
        );
        video_repository.videos.lock().unwrap().push(
            Video::create(
                PlaylistId::new("PL1").unwrap(),
                VideoId::new("vid1").unwrap(),
                "My Video",
                fixed_timestamp(),
            )
            .start_download(fixed_timestamp())
            .mark_downloaded(Quality::High, "My Video.mp4", fixed_timestamp()),
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        assert!(files.deleted_calls.lock().unwrap().is_empty());
        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored[0].status, VideoStatus::Downloaded);
    }
}
