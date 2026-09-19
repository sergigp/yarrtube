use crate::domain::channel::ChannelHandle;
use crate::domain::services::ChannelVideoReconciler;
use crate::domain::task::Task;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Runs every subsequent reconcile for a channel (the first one is
/// triggered by `subscribers::reconcile_on_channel_created` instead).
pub struct ReconcileChannelTask {
    channel_video_reconciler: ChannelVideoReconciler,
}

impl ReconcileChannelTask {
    pub fn new(channel_video_reconciler: ChannelVideoReconciler) -> Self {
        Self {
            channel_video_reconciler,
        }
    }
}

impl TaskHandler for ReconcileChannelTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let channel_id = Task::decode_reconcile_channel_payload(payload)?;
        let Ok(channel_id) = ChannelHandle::new(channel_id) else {
            return Ok(());
        };
        self.channel_video_reconciler.reconcile(channel_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, VideoLimit};
    use crate::domain::event::DomainEvent;
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::shared::{Quality, VideoId};
    use crate::domain::video::{Video, VideoStatus};
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, FakeChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::FakeChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::FakeVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::youtube_channel_videos_repository::{
        ChannelVideoListing, FakeChannelVideosRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn channel(video_limit: i64) -> Channel {
        Channel::create(
            ChannelHandle::new("@somechannel").unwrap(),
            "Some Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(video_limit).unwrap(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    #[allow(clippy::type_complexity)]
    fn handler_with(
        seed_channel: Option<Channel>,
        current_videos: Vec<ChannelVideoListing>,
    ) -> (
        ReconcileChannelTask,
        Arc<FakeEventPublisher>,
        Arc<FakeVideoRepository>,
        Arc<FakeChannelVideoRepository>,
        Arc<FakeTaskRepository>,
    ) {
        let (handler, events, videos, channel_videos, tasks, _files) = handler_with_files(
            seed_channel,
            current_videos,
            FakeVideoFileRepository::default(),
        );
        (handler, events, videos, channel_videos, tasks)
    }

    #[allow(clippy::type_complexity)]
    fn handler_with_files(
        seed_channel: Option<Channel>,
        current_videos: Vec<ChannelVideoListing>,
        video_file_repository: FakeVideoFileRepository,
    ) -> (
        ReconcileChannelTask,
        Arc<FakeEventPublisher>,
        Arc<FakeVideoRepository>,
        Arc<FakeChannelVideoRepository>,
        Arc<FakeTaskRepository>,
        Arc<FakeVideoFileRepository>,
    ) {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        if let Some(channel) = seed_channel {
            channel_repository.insert(&channel).unwrap();
        }
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let channel_video_repository = Arc::new(FakeChannelVideoRepository::default());
        let task_repository = Arc::new(FakeTaskRepository::default());
        let video_file_repository = Arc::new(video_file_repository);

        let thumbnail_fetcher = Arc::new(crate::domain::services::ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let reconciler = ChannelVideoReconciler::new(
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(current_videos)),
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
            ReconcileChannelTask::new(reconciler),
            event_publisher,
            video_repository,
            channel_video_repository,
            task_repository,
            video_file_repository,
        )
    }

    /// Seeds a stored video already a member of `@somechannel`, returning
    /// the created `Video` (its surrogate id is needed by several tests to
    /// build expected task payloads).
    fn seed_member(
        video_repository: &FakeVideoRepository,
        channel_video_repository: &FakeChannelVideoRepository,
        youtube_id: &str,
        title: &str,
        transform: impl FnOnce(Video) -> Video,
    ) -> Video {
        let video = transform(Video::create(
            VideoId::new(youtube_id).unwrap(),
            title,
            fixed_timestamp(),
        ));
        video_repository.videos.lock().unwrap().push(video.clone());
        channel_video_repository.register_youtube_id(&video.id, &video.youtube_id);
        channel_video_repository
            .channel_videos
            .lock()
            .unwrap()
            .push(crate::domain::channel_video::ChannelVideo::create(
                ChannelHandle::new("@somechannel").unwrap(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ));
        video
    }

    fn payload_for(channel_id: &str) -> String {
        Task::ReconcileChannel {
            channel_id: channel_id.to_string(),
        }
        .payload()
        .to_string()
    }

    #[test]
    fn it_should_no_op_when_the_channel_no_longer_exists() {
        let (handler, events, videos, channel_videos, tasks) = handler_with(None, Vec::new());

        handler.handle(&payload_for("@somechannel"), false).unwrap();

        assert!(events.published.lock().unwrap().is_empty());
        assert!(videos.videos.lock().unwrap().is_empty());
        assert!(channel_videos.channel_videos.lock().unwrap().is_empty());
        assert!(tasks.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_persist_a_video_within_the_top_n_and_publish_an_event() {
        let (handler, events, videos, _channel_videos, _tasks) = handler_with(
            Some(channel(10)),
            vec![ChannelVideoListing {
                youtube_id: "yt1".to_string(),
                title: "One".to_string(),
                position: 0,
            }],
        );

        handler.handle(&payload_for("@somechannel"), false).unwrap();

        let stored = videos.videos.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].status, VideoStatus::Pending);
        assert_eq!(
            *events.published.lock().unwrap(),
            vec![DomainEvent::VideoAddedToChannel {
                channel_id: "@somechannel".to_string(),
                video_id: stored[0].id.as_str().to_string(),
            }]
        );
    }

    #[test]
    fn it_should_refresh_the_title_and_position_of_an_already_stored_video() {
        let (handler, events, videos, channel_videos, _tasks) = handler_with(
            Some(channel(10)),
            vec![ChannelVideoListing {
                youtube_id: "yt1".to_string(),
                title: "Renamed".to_string(),
                position: 2,
            }],
        );
        let existing = Video::create(VideoId::new("yt1").unwrap(), "Original", fixed_timestamp());
        videos.videos.lock().unwrap().push(existing.clone());
        channel_videos.register_youtube_id(&existing.id, &VideoId::new("yt1").unwrap());
        channel_videos.channel_videos.lock().unwrap().push(
            crate::domain::channel_video::ChannelVideo::create(
                ChannelHandle::new("@somechannel").unwrap(),
                existing.id.clone(),
                0,
                fixed_timestamp(),
            ),
        );

        handler.handle(&payload_for("@somechannel"), false).unwrap();

        let stored = videos.videos.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].title, "Renamed");
        let stored_channel_videos = channel_videos.channel_videos.lock().unwrap();
        assert_eq!(stored_channel_videos.len(), 1);
        assert_eq!(stored_channel_videos[0].position, 2);
        assert!(events.published.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_evict_a_video_that_ages_out_of_the_top_n() {
        let (handler, events, videos, channel_videos, _tasks) =
            handler_with(Some(channel(1)), Vec::new());
        let existing = Video::create(VideoId::new("yt_old").unwrap(), "Old", fixed_timestamp());
        videos.videos.lock().unwrap().push(existing.clone());
        channel_videos.channel_videos.lock().unwrap().push(
            crate::domain::channel_video::ChannelVideo::create(
                ChannelHandle::new("@somechannel").unwrap(),
                existing.id.clone(),
                0,
                fixed_timestamp(),
            ),
        );
        channel_videos.register_youtube_id(&existing.id, &VideoId::new("yt_old").unwrap());

        handler.handle(&payload_for("@somechannel"), false).unwrap();

        assert!(videos.videos.lock().unwrap().is_empty());
        assert!(channel_videos.channel_videos.lock().unwrap().is_empty());
        assert_eq!(
            *events.published.lock().unwrap(),
            vec![DomainEvent::VideoRemovedFromChannel {
                channel_id: "@somechannel".to_string(),
                video_id: existing.id.as_str().to_string(),
                title: "Old".to_string(),
                filename: None,
                thumbnail_filename: None,
                was_downloaded: false,
            }]
        );
    }

    #[test]
    fn it_should_leave_stored_videos_untouched_when_yt_dlp_fails() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        channel_repository.insert(&channel(10)).unwrap();
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let channel_video_repository = Arc::new(FakeChannelVideoRepository::default());
        let existing = Video::create(VideoId::new("yt_old").unwrap(), "Old", fixed_timestamp());
        video_repository
            .videos
            .lock()
            .unwrap()
            .push(existing.clone());
        channel_video_repository
            .channel_videos
            .lock()
            .unwrap()
            .push(crate::domain::channel_video::ChannelVideo::create(
                ChannelHandle::new("@somechannel").unwrap(),
                existing.id.clone(),
                0,
                fixed_timestamp(),
            ));
        let thumbnail_fetcher = Arc::new(crate::domain::services::ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let reconciler = ChannelVideoReconciler::new(
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::failing()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
            event_publisher.clone(),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let handler = ReconcileChannelTask::new(reconciler);

        let result = handler.handle(&payload_for("@somechannel"), false);

        assert!(result.is_err());
        assert_eq!(video_repository.videos.lock().unwrap().len(), 1);
        assert_eq!(
            channel_video_repository
                .channel_videos
                .lock()
                .unwrap()
                .len(),
            1
        );
        assert!(event_publisher.published.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_always_schedule_the_next_reconcile_even_with_no_changes() {
        let (handler, _events, _videos, _channel_videos, tasks) =
            handler_with(Some(channel(10)), Vec::new());

        handler.handle(&payload_for("@somechannel"), false).unwrap();

        let scheduled = tasks.scheduled.lock().unwrap();
        assert_eq!(scheduled.len(), 1);
        assert_eq!(
            scheduled[0].0,
            Task::ReconcileChannel {
                channel_id: "@somechannel".to_string()
            }
        );
        assert_eq!(
            scheduled[0].1,
            fixed_timestamp() + chrono::Duration::seconds(3600)
        );
    }

    #[test]
    fn it_should_no_op_when_the_payload_channel_id_is_invalid() {
        let (handler, events, videos, _channel_videos, tasks) =
            handler_with(Some(channel(10)), Vec::new());

        handler.handle(&payload_for(""), false).unwrap();

        assert!(events.published.lock().unwrap().is_empty());
        assert!(videos.videos.lock().unwrap().is_empty());
        assert!(tasks.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let (handler, ..) = handler_with(Some(channel(10)), Vec::new());

        assert!(handler.handle("not json", false).is_err());
    }

    #[test]
    fn it_should_heal_a_downloaded_video_whose_file_is_missing() {
        let (handler, _events, videos, channel_videos, tasks, _files) = handler_with_files(
            Some(channel(10)),
            vec![ChannelVideoListing {
                youtube_id: "yt1".to_string(),
                title: "My Video".to_string(),
                position: 0,
            }],
            FakeVideoFileRepository::with_listing(Vec::new()),
        );
        let video = seed_member(&videos, &channel_videos, "yt1", "My Video", |v| {
            v.start_download(fixed_timestamp()).mark_downloaded(
                Quality::High,
                "My Video.mp4",
                Some("My Video.jpg".to_string()),
                None,
                fixed_timestamp(),
            )
        });

        handler.handle(&payload_for("@somechannel"), false).unwrap();

        let stored = videos.videos.lock().unwrap();
        assert_eq!(stored[0].status, VideoStatus::Pending);
        assert_eq!(stored[0].filename, None);
        assert_eq!(stored[0].thumbnail_filename, None);
        assert_eq!(stored[0].quality, None);

        let scheduled = tasks.scheduled.lock().unwrap();
        assert!(scheduled.iter().any(|(task, _)| *task
            == Task::DownloadVideo {
                video_id: video.id.as_str().to_string(),
                quality: "high".to_string(),
                output_dir: "/videos/creators/somechannel".to_string(),
            }));
    }

    #[test]
    fn it_should_heal_a_downloaded_video_whose_file_is_present_but_not_mp4() {
        let (handler, _events, videos, channel_videos, tasks, _files) = handler_with_files(
            Some(channel(10)),
            vec![ChannelVideoListing {
                youtube_id: "yt1".to_string(),
                title: "My Video".to_string(),
                position: 0,
            }],
            FakeVideoFileRepository::with_listing(vec!["My Video.webm".to_string()]),
        );
        let video = seed_member(&videos, &channel_videos, "yt1", "My Video", |v| {
            v.start_download(fixed_timestamp()).mark_downloaded(
                Quality::High,
                "My Video.webm",
                None,
                None,
                fixed_timestamp(),
            )
        });

        handler.handle(&payload_for("@somechannel"), false).unwrap();

        let stored = videos.videos.lock().unwrap();
        assert_eq!(stored[0].status, VideoStatus::Pending);
        assert_eq!(stored[0].filename, None);
        assert_eq!(stored[0].quality, None);

        let scheduled = tasks.scheduled.lock().unwrap();
        assert!(scheduled.iter().any(|(task, _)| *task
            == Task::DownloadVideo {
                video_id: video.id.as_str().to_string(),
                quality: "high".to_string(),
                output_dir: "/videos/creators/somechannel".to_string(),
            }));
    }

    #[test]
    fn it_should_delete_an_orphaned_file() {
        let (handler, _events, _videos, _channel_videos, _tasks, files) = handler_with_files(
            Some(channel(10)),
            Vec::new(),
            FakeVideoFileRepository::with_listing(vec!["orphan.mp4".to_string()]),
        );

        handler.handle(&payload_for("@somechannel"), false).unwrap();

        let calls = files.deleted_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, "orphan.mp4");
    }

    #[test]
    fn it_should_recover_a_permanently_errored_video() {
        let (handler, _events, videos, channel_videos, tasks, _files) = handler_with_files(
            Some(channel(10)),
            vec![ChannelVideoListing {
                youtube_id: "yt1".to_string(),
                title: "My Video".to_string(),
                position: 0,
            }],
            FakeVideoFileRepository::with_listing(Vec::new()),
        );
        let video = seed_member(&videos, &channel_videos, "yt1", "My Video", |v| {
            v.start_download(fixed_timestamp())
                .mark_errored(fixed_timestamp())
        });

        handler.handle(&payload_for("@somechannel"), false).unwrap();

        let stored = videos.videos.lock().unwrap();
        assert_eq!(stored[0].status, VideoStatus::Pending);
        assert_eq!(stored[0].filename, None);
        assert_eq!(stored[0].quality, None);

        let scheduled = tasks.scheduled.lock().unwrap();
        assert!(scheduled.iter().any(|(task, _)| *task
            == Task::DownloadVideo {
                video_id: video.id.as_str().to_string(),
                quality: "high".to_string(),
                output_dir: "/videos/creators/somechannel".to_string(),
            }));
    }

    #[test]
    fn it_should_leave_a_matching_file_alone() {
        let (handler, _events, videos, channel_videos, _tasks, files) = handler_with_files(
            Some(channel(10)),
            vec![ChannelVideoListing {
                youtube_id: "yt1".to_string(),
                title: "My Video".to_string(),
                position: 0,
            }],
            FakeVideoFileRepository::with_listing(vec!["My Video.mp4".to_string()]),
        );
        seed_member(&videos, &channel_videos, "yt1", "My Video", |v| {
            v.start_download(fixed_timestamp()).mark_downloaded(
                Quality::High,
                "My Video.mp4",
                None,
                None,
                fixed_timestamp(),
            )
        });

        handler.handle(&payload_for("@somechannel"), false).unwrap();

        assert!(files.deleted_calls.lock().unwrap().is_empty());
        let stored = videos.videos.lock().unwrap();
        assert_eq!(stored[0].status, VideoStatus::Downloaded);
    }

    #[test]
    fn it_should_leave_a_matching_thumbnail_file_alone() {
        let (handler, _events, videos, channel_videos, _tasks, files) = handler_with_files(
            Some(channel(10)),
            vec![ChannelVideoListing {
                youtube_id: "yt1".to_string(),
                title: "My Video".to_string(),
                position: 0,
            }],
            FakeVideoFileRepository::with_listing(vec![
                "My Video.mp4".to_string(),
                "My Video.jpg".to_string(),
            ]),
        );
        seed_member(&videos, &channel_videos, "yt1", "My Video", |v| {
            v.start_download(fixed_timestamp()).mark_downloaded(
                Quality::High,
                "My Video.mp4",
                Some("My Video.jpg".to_string()),
                None,
                fixed_timestamp(),
            )
        });

        handler.handle(&payload_for("@somechannel"), false).unwrap();

        assert!(files.deleted_calls.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_delete_an_unrecorded_stray_thumbnail_file() {
        let (handler, _events, videos, channel_videos, _tasks, files) = handler_with_files(
            Some(channel(10)),
            vec![ChannelVideoListing {
                youtube_id: "yt1".to_string(),
                title: "My Video".to_string(),
                position: 0,
            }],
            FakeVideoFileRepository::with_listing(vec![
                "My Video.mp4".to_string(),
                "stray.jpg".to_string(),
            ]),
        );
        seed_member(&videos, &channel_videos, "yt1", "My Video", |v| {
            v.start_download(fixed_timestamp()).mark_downloaded(
                Quality::High,
                "My Video.mp4",
                None,
                None,
                fixed_timestamp(),
            )
        });

        handler.handle(&payload_for("@somechannel"), false).unwrap();

        let calls = files.deleted_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, "stray.jpg");
    }
}
