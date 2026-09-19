use crate::domain::services::VideoReconciler;
use crate::domain::shared::PlaylistId;
use crate::domain::task::Task;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Runs every subsequent reconcile for a playlist (the first one is
/// triggered by `subscribers::reconcile_on_playlist_created` instead).
pub struct ReconcilePlaylistTask {
    video_reconciler: VideoReconciler,
}

impl ReconcilePlaylistTask {
    pub fn new(video_reconciler: VideoReconciler) -> Self {
        Self { video_reconciler }
    }
}

impl TaskHandler for ReconcilePlaylistTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let playlist_id = Task::decode_reconcile_playlist_payload(payload)?;
        let Ok(playlist_id) = PlaylistId::new(playlist_id) else {
            return Ok(());
        };
        self.video_reconciler.reconcile(playlist_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::DomainEvent;
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::shared::{Quality, VideoId};
    use crate::domain::video::{Video, VideoStatus};
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::{
        FakePlaylistVideoRepository, PlaylistVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::FakeVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::{
        FakeYoutubePlaylistItemsRepository, YoutubePlaylistItem,
    };

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

    #[allow(clippy::type_complexity)]
    fn handler_with(
        kind: PlaylistKind,
        current_videos: Vec<YoutubePlaylistItem>,
        video_file_repository: FakeVideoFileRepository,
    ) -> (
        ReconcilePlaylistTask,
        Arc<FakeEventPublisher>,
        Arc<FakeVideoRepository>,
        Arc<FakePlaylistVideoRepository>,
        Arc<FakeTaskRepository>,
        Arc<FakeVideoFileRepository>,
    ) {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert(&Playlist::create(
                playlist_id(),
                PlaylistName::new("My Playlist").unwrap(),
                PlaylistPath::new("my-playlist").unwrap(),
                Quality::High,
                kind,
                fixed_timestamp(),
            ))
            .unwrap();
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let task_repository = Arc::new(FakeTaskRepository::default());
        let video_file_repository = Arc::new(video_file_repository);

        let thumbnail_fetcher = Arc::new(crate::domain::services::ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_reconciler = VideoReconciler::new(
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository {
                videos: std::sync::Mutex::new(current_videos),
            }),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
            event_publisher.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );

        (
            ReconcilePlaylistTask::new(video_reconciler),
            event_publisher,
            video_repository,
            playlist_video_repository,
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

    /// Seeds a stored video already a member of `playlist_id()` under
    /// `youtube_id`, returning the created `Video` (its surrogate id is
    /// needed by several tests to build expected events).
    fn seed_member(
        video_repository: &FakeVideoRepository,
        playlist_video_repository: &FakePlaylistVideoRepository,
        youtube_id: &str,
        title: &str,
        transform: impl FnOnce(Video) -> Video,
    ) -> Video {
        let video = transform(Video::create(
            VideoId::new(youtube_id).unwrap(),
            title,
            fixed_timestamp(),
        ));
        video_repository.save(&video).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create_with_position(
                playlist_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        playlist_video_repository.register_youtube_id(&video.id, &video.youtube_id);
        video
    }

    #[test]
    fn it_should_no_op_when_the_playlist_no_longer_exists() {
        let (handler, event_publisher, video_repository, _playlist_videos, task_repository, _files) =
            handler_with(
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
        let (handler, event_publisher, video_repository, _playlist_videos, task_repository, _files) =
            handler_with(
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
        let (handler, event_publisher, video_repository, _playlist_videos, _tasks, _files) =
            handler_with(
                PlaylistKind::YoutubeLinked,
                vec![YoutubePlaylistItem {
                    video_id: "vid1".to_string(),
                    title: "One".to_string(),
                    position: 0,
                }],
                FakeVideoFileRepository::default(),
            );

        handler.handle(&payload_for("PL1"), false).unwrap();

        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].status, VideoStatus::Pending);
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::VideoAddedToPlaylist {
                playlist_id: "PL1".to_string(),
                video_id: stored[0].id.as_str().to_string(),
            }]
        );
    }

    #[test]
    fn it_should_leave_an_existing_videos_status_unchanged_while_refreshing_its_title() {
        let (handler, event_publisher, video_repository, playlist_videos, _tasks, _files) =
            handler_with(
                PlaylistKind::YoutubeLinked,
                vec![YoutubePlaylistItem {
                    video_id: "vid1".to_string(),
                    title: "Renamed".to_string(),
                    position: 0,
                }],
                FakeVideoFileRepository::default(),
            );
        seed_member(
            &video_repository,
            &playlist_videos,
            "vid1",
            "Original",
            |v| v.start_download(fixed_timestamp()),
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].title, "Renamed");
        assert_eq!(stored[0].status, VideoStatus::InProgress);
        assert!(event_publisher.published.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_update_a_stored_videos_position_on_a_later_reconcile_pass() {
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
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let youtube_playlist_items_repository = Arc::new(FakeYoutubePlaylistItemsRepository {
            videos: std::sync::Mutex::new(vec![YoutubePlaylistItem {
                video_id: "vid1".to_string(),
                title: "One".to_string(),
                position: 0,
            }]),
        });
        let thumbnail_fetcher = Arc::new(crate::domain::services::ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_reconciler = VideoReconciler::new(
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            youtube_playlist_items_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );

        video_reconciler.force_reconcile(playlist_id()).unwrap();
        let stored = playlist_video_repository
            .list_for_playlist(&playlist_id())
            .unwrap();
        assert_eq!(stored[0].position, Some(0));
        // The fake's find-by-youtube-id lookup mirrors the real
        // repository's join with `videos`, but (unlike SQL) needs to be
        // told the mapping explicitly since it has no shared video store.
        let created_video = video_repository.find(&stored[0].video_id).unwrap().unwrap();
        playlist_video_repository.register_youtube_id(&created_video.id, &created_video.youtube_id);

        *youtube_playlist_items_repository.videos.lock().unwrap() = vec![YoutubePlaylistItem {
            video_id: "vid1".to_string(),
            title: "One".to_string(),
            position: 2,
        }];
        video_reconciler.force_reconcile(playlist_id()).unwrap();

        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id())
                .unwrap()[0]
                .position,
            Some(2)
        );
    }

    #[test]
    fn it_should_delete_videos_no_longer_present_on_youtube() {
        let (handler, _events, video_repository, playlist_videos, _tasks, _files) = handler_with(
            PlaylistKind::YoutubeLinked,
            Vec::new(),
            FakeVideoFileRepository::default(),
        );
        seed_member(&video_repository, &playlist_videos, "vid1", "Stale", |v| v);

        handler.handle(&payload_for("PL1"), false).unwrap();

        assert!(video_repository.videos.lock().unwrap().is_empty());
        assert!(
            playlist_videos
                .list_for_playlist(&playlist_id())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn it_should_publish_a_video_deleted_event_with_the_correct_was_downloaded_per_video() {
        let (handler, event_publisher, video_repository, playlist_videos, _tasks, _files) =
            handler_with(
                PlaylistKind::YoutubeLinked,
                Vec::new(),
                FakeVideoFileRepository::default(),
            );
        let downloaded = seed_member(
            &video_repository,
            &playlist_videos,
            "vid1",
            "Downloaded Video",
            |v| {
                v.start_download(fixed_timestamp()).mark_downloaded(
                    Quality::High,
                    "Downloaded Video.mp4",
                    Some("Downloaded Video.jpg".to_string()),
                    None,
                    fixed_timestamp(),
                )
            },
        );
        let pending = seed_member(
            &video_repository,
            &playlist_videos,
            "vid2",
            "Pending Video",
            |v| v,
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        let published = event_publisher.published.lock().unwrap();
        assert_eq!(
            *published,
            vec![
                DomainEvent::VideoRemovedFromPlaylist {
                    playlist_id: "PL1".to_string(),
                    video_id: downloaded.id.as_str().to_string(),
                    title: "Downloaded Video".to_string(),
                    filename: Some("Downloaded Video.mp4".to_string()),
                    thumbnail_filename: Some("Downloaded Video.jpg".to_string()),
                    was_downloaded: true,
                },
                DomainEvent::VideoRemovedFromPlaylist {
                    playlist_id: "PL1".to_string(),
                    video_id: pending.id.as_str().to_string(),
                    title: "Pending Video".to_string(),
                    filename: None,
                    thumbnail_filename: None,
                    was_downloaded: false,
                },
            ]
        );
    }

    #[test]
    fn it_should_always_schedule_the_next_reconcile_even_with_no_changes() {
        let (handler, _events, _videos, _playlist_videos, task_repository, _files) = handler_with(
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
        let (handler, event_publisher, video_repository, _playlist_videos, task_repository, _files) =
            handler_with(
                PlaylistKind::Custom,
                vec![YoutubePlaylistItem {
                    video_id: "vid1".to_string(),
                    title: "One".to_string(),
                    position: 0,
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
        let (handler, _events, video_repository, playlist_videos, task_repository, _files) =
            handler_with(
                PlaylistKind::Custom,
                Vec::new(),
                FakeVideoFileRepository::with_listing(Vec::new()),
            );
        seed_member(
            &video_repository,
            &playlist_videos,
            "vid1",
            "My Video",
            |v| {
                v.start_download(fixed_timestamp()).mark_downloaded(
                    Quality::High,
                    "My Video.mp4",
                    Some("My Video.jpg".to_string()),
                    None,
                    fixed_timestamp(),
                )
            },
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored[0].status, VideoStatus::Pending);
        assert_eq!(stored[0].filename, None);
        assert_eq!(stored[0].thumbnail_filename, None);
        assert_eq!(stored[0].quality, None);

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert!(scheduled.iter().any(|(task, _)| *task
            == Task::DownloadVideo {
                video_id: stored[0].id.as_str().to_string(),
                quality: "high".to_string(),
                output_dir: "/videos/my-playlist".to_string(),
            }));
    }

    #[test]
    fn it_should_heal_a_downloaded_video_whose_file_is_present_but_not_mp4() {
        let (handler, _events, video_repository, playlist_videos, task_repository, _files) =
            handler_with(
                PlaylistKind::Custom,
                Vec::new(),
                FakeVideoFileRepository::with_listing(vec!["My Video.webm".to_string()]),
            );
        seed_member(
            &video_repository,
            &playlist_videos,
            "vid1",
            "My Video",
            |v| {
                v.start_download(fixed_timestamp()).mark_downloaded(
                    Quality::High,
                    "My Video.webm",
                    None,
                    None,
                    fixed_timestamp(),
                )
            },
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored[0].status, VideoStatus::Pending);
        assert_eq!(stored[0].filename, None);
        assert_eq!(stored[0].quality, None);

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert!(scheduled.iter().any(|(task, _)| *task
            == Task::DownloadVideo {
                video_id: stored[0].id.as_str().to_string(),
                quality: "high".to_string(),
                output_dir: "/videos/my-playlist".to_string(),
            }));
    }

    #[test]
    fn it_should_delete_an_orphaned_file() {
        let (handler, _events, _videos, _playlist_videos, _tasks, files) = handler_with(
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
    fn it_should_recover_a_permanently_errored_video() {
        let (handler, _events, video_repository, playlist_videos, task_repository, _files) =
            handler_with(
                PlaylistKind::Custom,
                Vec::new(),
                FakeVideoFileRepository::with_listing(Vec::new()),
            );
        seed_member(
            &video_repository,
            &playlist_videos,
            "vid1",
            "My Video",
            |v| {
                v.start_download(fixed_timestamp())
                    .mark_errored(fixed_timestamp())
            },
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored[0].status, VideoStatus::Pending);
        assert_eq!(stored[0].filename, None);
        assert_eq!(stored[0].quality, None);

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert!(scheduled.iter().any(|(task, _)| *task
            == Task::DownloadVideo {
                video_id: stored[0].id.as_str().to_string(),
                quality: "high".to_string(),
                output_dir: "/videos/my-playlist".to_string(),
            }));
    }

    #[test]
    fn it_should_recover_a_video_again_after_it_errors_again_post_recovery() {
        let (handler, _events, video_repository, playlist_videos, _tasks, _files) = handler_with(
            PlaylistKind::Custom,
            Vec::new(),
            FakeVideoFileRepository::with_listing(Vec::new()),
        );
        seed_member(
            &video_repository,
            &playlist_videos,
            "vid1",
            "My Video",
            |v| {
                v.start_download(fixed_timestamp())
                    .mark_errored(fixed_timestamp())
            },
        );

        handler.handle(&payload_for("PL1"), false).unwrap();
        {
            let mut stored = video_repository.videos.lock().unwrap();
            assert_eq!(stored[0].status, VideoStatus::Pending);
            stored[0] = stored[0].clone().start_download(fixed_timestamp());
            stored[0] = stored[0].clone().mark_errored(fixed_timestamp());
        }

        handler.handle(&payload_for("PL1"), false).unwrap();

        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored[0].status, VideoStatus::Pending);
        assert_eq!(stored[0].filename, None);
        assert_eq!(stored[0].quality, None);
    }

    #[test]
    fn it_should_leave_a_matching_file_alone() {
        let (handler, _events, video_repository, playlist_videos, _tasks, files) = handler_with(
            PlaylistKind::Custom,
            Vec::new(),
            FakeVideoFileRepository::with_listing(vec!["My Video.mp4".to_string()]),
        );
        seed_member(
            &video_repository,
            &playlist_videos,
            "vid1",
            "My Video",
            |v| {
                v.start_download(fixed_timestamp()).mark_downloaded(
                    Quality::High,
                    "My Video.mp4",
                    None,
                    None,
                    fixed_timestamp(),
                )
            },
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        assert!(files.deleted_calls.lock().unwrap().is_empty());
        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored[0].status, VideoStatus::Downloaded);
    }

    #[test]
    fn it_should_leave_a_matching_thumbnail_file_alone() {
        let (handler, _events, video_repository, playlist_videos, _tasks, files) = handler_with(
            PlaylistKind::Custom,
            Vec::new(),
            FakeVideoFileRepository::with_listing(vec![
                "My Video.mp4".to_string(),
                "My Video.jpg".to_string(),
            ]),
        );
        seed_member(
            &video_repository,
            &playlist_videos,
            "vid1",
            "My Video",
            |v| {
                v.start_download(fixed_timestamp()).mark_downloaded(
                    Quality::High,
                    "My Video.mp4",
                    Some("My Video.jpg".to_string()),
                    None,
                    fixed_timestamp(),
                )
            },
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        assert!(files.deleted_calls.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_delete_an_unrecorded_stray_thumbnail_file() {
        let (handler, _events, video_repository, playlist_videos, _tasks, files) = handler_with(
            PlaylistKind::Custom,
            Vec::new(),
            FakeVideoFileRepository::with_listing(vec![
                "My Video.mp4".to_string(),
                "stray.jpg".to_string(),
            ]),
        );
        seed_member(
            &video_repository,
            &playlist_videos,
            "vid1",
            "My Video",
            |v| {
                v.start_download(fixed_timestamp()).mark_downloaded(
                    Quality::High,
                    "My Video.mp4",
                    None,
                    None,
                    fixed_timestamp(),
                )
            },
        );

        handler.handle(&payload_for("PL1"), false).unwrap();

        let calls = files.deleted_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, "stray.jpg");
    }
}
