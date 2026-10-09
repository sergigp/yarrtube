use crate::domain::channel::ChannelHandle;
use crate::domain::services::{ChannelVideoReconciler, ChannelVideoReconcilerApi};
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
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::channel_video::{VideoAddedToChannel, VideoRemovedFromChannel};
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::services::InternalVideoReconciler;
    use crate::domain::services::MetadataGenerator;
    use crate::domain::services::ThumbnailFetcher;
    use crate::domain::shared::Quality;
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::domain::video::Video;
    use crate::domain::video::{VideoId, VideoRecordId, VideoStatus};
    use crate::domain::video_metadata::VideoMetadata;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, SqliteChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, SqliteChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::{
        SqliteVideoMetadataRepository, VideoMetadataRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_channel_videos_repository::{
        ChannelVideoListing, FakeChannelVideosRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::{
        FakeYoutubeMetadataRepository, YoutubeMetadata,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use crate::infrastructure::shared::ytdlp::FetchedThumbnail;
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::sync::Arc;

    #[test]
    fn it_should_skip_if_channel_is_gone() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            vec![]
        );
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_add_new_videos_within_the_limit() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "One", 0),
            ])),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        let videos = video_repository.list().unwrap();
        let video_id = videos[0].id.clone();
        assert_eq!(
            videos,
            vec![Video {
                id: video_id.clone(),
                ..Video::create(VideoId::new("yt1").unwrap(), "One", fixed_timestamp())
            }]
        );
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            vec![ChannelVideo {
                id: 1,
                ..ChannelVideo::create(
                    handle("@somechannel"),
                    video_id.clone(),
                    0,
                    fixed_timestamp()
                )
            }]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![next_reconcile(1)]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoAddedToChannel(VideoAddedToChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: video_id.as_str().to_string(),
                })
            )]
        );
    }

    #[test]
    fn it_should_refresh_existing_videos() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let existing = Video::create(VideoId::new("yt1").unwrap(), "Original", fixed_timestamp());
        let existing_channel_video = save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &existing,
        );
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "Renamed", 2),
            ])),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                title: "Renamed".to_string(),
                ..existing
            }]
        );
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            vec![ChannelVideo {
                position: 2,
                ..existing_channel_video
            }]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_remove_videos_beyond_the_limit() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        channel_repository
            .insert(&Channel {
                video_limit: VideoLimit::new(1).unwrap(),
                ..channel("@somechannel")
            })
            .unwrap();
        let existing = Video::create(VideoId::new("yt_old").unwrap(), "Old", fixed_timestamp());
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &existing,
        );
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            vec![]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoRemovedFromChannel(VideoRemovedFromChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: existing.id.as_str().to_string(),
                    title: "Old".to_string(),
                    filename: None,
                    thumbnail_filename: None,
                    was_downloaded: false,
                })
            )]
        );
    }

    #[test]
    fn it_should_remove_the_oldest_videos_after_the_video_limit_is_lowered() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let original = Channel {
            video_limit: VideoLimit::new(3).unwrap(),
            ..channel("@somechannel")
        };
        channel_repository.insert(&original).unwrap();
        let newest = Video::create(VideoId::new("yt1").unwrap(), "One", fixed_timestamp());
        let middle = Video::create(VideoId::new("yt2").unwrap(), "Two", fixed_timestamp());
        let oldest = Video::create(VideoId::new("yt3").unwrap(), "Three", fixed_timestamp());
        let newest_channel_video = save_channel_video_at(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &newest,
            0,
        );
        let middle_channel_video = save_channel_video_at(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &middle,
            1,
        );
        save_channel_video_at(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &oldest,
            2,
        );
        channel_repository
            .update(&original.with_video_limit(VideoLimit::new(2).unwrap()))
            .unwrap();
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "One", 0),
                listed_video("yt2", "Two", 1),
                listed_video("yt3", "Three", 2),
            ])),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![newest.clone(), middle.clone()]
        );
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            vec![newest_channel_video, middle_channel_video]
        );
        // The kept videos are still pending, so the pass queues their downloads.
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                download_video_task(1, &newest.id),
                download_video_task(2, &middle.id),
                fetch_thumbnail_task(3, &newest.id),
                fetch_thumbnail_task(4, &middle.id),
                next_reconcile(5),
            ]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoRemovedFromChannel(VideoRemovedFromChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: oldest.id.as_str().to_string(),
                    title: "Three".to_string(),
                    filename: None,
                    thumbnail_filename: None,
                    was_downloaded: false,
                })
            )]
        );
    }

    #[test]
    fn it_should_add_the_next_most_recent_videos_after_the_video_limit_is_raised() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let original = Channel {
            video_limit: VideoLimit::new(1).unwrap(),
            ..channel("@somechannel")
        };
        channel_repository.insert(&original).unwrap();
        let newest = Video::create(VideoId::new("yt1").unwrap(), "One", fixed_timestamp());
        let newest_channel_video = save_channel_video_at(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &newest,
            0,
        );
        channel_repository
            .update(&original.with_video_limit(VideoLimit::new(2).unwrap()))
            .unwrap();
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "One", 0),
                listed_video("yt2", "Two", 1),
                listed_video("yt3", "Three", 2),
            ])),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        let videos = video_repository.list().unwrap();
        let added_id = videos
            .iter()
            .find(|video| video.youtube_id.as_str() == "yt2")
            .unwrap()
            .id
            .clone();
        assert_eq!(
            videos,
            vec![
                newest.clone(),
                Video {
                    id: added_id.clone(),
                    ..Video::create(VideoId::new("yt2").unwrap(), "Two", fixed_timestamp())
                },
            ]
        );
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            vec![
                newest_channel_video,
                ChannelVideo {
                    id: 2,
                    ..ChannelVideo::create(
                        handle("@somechannel"),
                        added_id.clone(),
                        1,
                        fixed_timestamp()
                    )
                },
            ]
        );
        // The kept video is still pending, so the pass queues its download;
        // the added one's is triggered by its `VideoAddedToChannel` instead.
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                download_video_task(1, &newest.id),
                fetch_thumbnail_task(2, &newest.id),
                next_reconcile(3),
            ]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoAddedToChannel(VideoAddedToChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: added_id.as_str().to_string(),
                })
            )]
        );
    }

    #[test]
    fn it_should_keep_videos_if_listing_fails() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let existing = Video::create(VideoId::new("yt_old").unwrap(), "Old", fixed_timestamp());
        let existing_channel_video = save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &existing,
        );
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::failing()),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![existing.clone()]);
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            vec![existing_channel_video]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                download_video_task(1, &existing.id),
                fetch_thumbnail_task(2, &existing.id),
                next_reconcile(3),
            ]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_always_schedule_the_next_reconcile() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![next_reconcile(1)]
        );
    }

    #[test]
    fn it_should_skip_if_invalid_channel_id_provided() {
        let result = run(&any_task(), &payload_for(""));

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let result = run(&any_task(), "not json");

        assert_eq!(
            result,
            Err("invalid reconcile_channel payload: expected ident at line 1 column 2".to_string())
        );
    }

    #[test]
    fn it_should_redownload_videos_with_missing_file() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = Video {
            duration_seconds: Some(223),
            ..downloaded_video("My Video.mp4", Some("My Video.jpg"))
        }
        .mark_watched(watched_timestamp());
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &video,
        );
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "My Video", 0),
            ])),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                status: VideoStatus::Pending,
                quality: None,
                filename: None,
                thumbnail_filename: None,
                duration_seconds: None,
                updated_at: fixed_timestamp(),
                synced_at: None,
                ..video.clone()
            }]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(
                    1,
                    &Task::DownloadVideo {
                        video_id: video.id.as_str().to_string(),
                        quality: "high".to_string(),
                        output_dir: "/videos/creators/somechannel".to_string(),
                    },
                    fixed_timestamp(),
                ),
                next_reconcile(2),
            ]
        );
    }

    #[test]
    fn it_should_keep_the_folder_of_a_download_in_progress_if_video_renamed() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video".to_string(),
        ]));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = my_video().start_download(fixed_timestamp());
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &video,
        );
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "Renamed", 0),
            ])),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                title: "Renamed".to_string(),
                ..video
            }]
        );
    }

    #[test]
    fn it_should_generate_missing_metadata() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let videos_root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(videos_root.path().join("creators/somechannel/My Video")).unwrap();
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_video("My Video/My Video.mp4", None);
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &video,
        );
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository,
            video_repository.clone(),
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "My Video", 0),
            ])),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(MetadataGenerator::new(
                    Arc::new(FakeYoutubeMetadataRepository {
                        metadata: Some(youtube_metadata("My Video")),
                    }),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                video_metadata_repository.clone(),
                task_repository.clone(),
                Arc::new(FakeVideoFileRepository::with_file_exists(true)),
                Arc::new(ThumbnailFetcher::new(
                    video_repository.clone(),
                    Arc::new(FakeVideoDownloaderRepository::default()),
                    task_repository.clone(),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(FixedClock(fixed_timestamp())),
                videos_root.path().to_string_lossy(),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
        ));

        let result = run(&task, &payload_for("@somechannel"));

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
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(
                    1,
                    &Task::FetchThumbnail {
                        video_id: video.id.as_str().to_string(),
                        output_dir: videos_root
                            .path()
                            .join("creators/somechannel")
                            .to_string_lossy()
                            .to_string(),
                    },
                    fixed_timestamp(),
                ),
                next_reconcile(2),
            ]
        );
    }

    #[test]
    fn it_should_not_fetch_thumbnails_when_adding_new_videos() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_thumbnail_result(Some(fetched_thumbnail())),
        );
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository,
            video_repository.clone(),
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "One", 0),
                listed_video("yt2", "Two", 1),
            ])),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(MetadataGenerator::new(
                    Arc::new(FakeYoutubeMetadataRepository::default()),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                task_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(ThumbnailFetcher::new(
                    video_repository.clone(),
                    video_downloader_repository.clone(),
                    task_repository.clone(),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        let videos = video_repository.list().unwrap();
        let (first_id, second_id) = (videos[0].id.clone(), videos[1].id.clone());
        assert_eq!(
            videos,
            vec![
                Video {
                    id: first_id.clone(),
                    ..Video::create(VideoId::new("yt1").unwrap(), "One", fixed_timestamp())
                },
                Video {
                    id: second_id.clone(),
                    ..Video::create(VideoId::new("yt2").unwrap(), "Two", fixed_timestamp())
                },
            ]
        );
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![
                pending_event(
                    1,
                    DomainEvent::VideoAddedToChannel(VideoAddedToChannel {
                        channel_id: "@somechannel".to_string(),
                        video_id: first_id.as_str().to_string(),
                    })
                ),
                pending_event(
                    2,
                    DomainEvent::VideoAddedToChannel(VideoAddedToChannel {
                        channel_id: "@somechannel".to_string(),
                        video_id: second_id.as_str().to_string(),
                    })
                ),
            ]
        );
    }

    #[test]
    fn it_should_not_schedule_a_thumbnail_fetch_for_a_video_added_in_the_same_pass() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "One", 0),
            ])),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![next_reconcile(1)]
        );
    }

    /// Builds a reconciler around the repositories and fakes a test seeds,
    /// configures or asserts; the remaining ports (YouTube metadata, video
    /// metadata, thumbnails) are ones these tests don't observe — the ones
    /// that do build the reconciler inline. Events go to `db`'s outbox table.
    fn channel_video_reconciler(
        db: &TestDatabase,
        channel_repository: Arc<SqliteChannelRepository>,
        video_repository: Arc<SqliteVideoRepository>,
        channel_video_repository: Arc<SqliteChannelVideoRepository>,
        channel_videos_repository: Arc<FakeChannelVideosRepository>,
        task_repository: Arc<SqliteTaskRepository>,
        video_file_repository: Arc<FakeVideoFileRepository>,
    ) -> ChannelVideoReconciler {
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        ChannelVideoReconciler::new(
            channel_repository,
            video_repository.clone(),
            channel_video_repository,
            channel_videos_repository,
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository,
                Arc::new(MetadataGenerator::new(
                    Arc::new(FakeYoutubeMetadataRepository::default()),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                task_repository.clone(),
                video_file_repository,
                thumbnail_fetcher,
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
        )
    }

    /// A task for tests whose payload is rejected before reaching the
    /// reconciler. Its repositories sit on an unmigrated in-memory database,
    /// so a payload that wrongly got through would fail loudly instead of
    /// passing.
    fn any_task() -> ReconcileChannelTask {
        let video_repository = Arc::new(SqliteVideoRepository::new(unused_connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            unused_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        ReconcileChannelTask::new(ChannelVideoReconciler::new(
            Arc::new(SqliteChannelRepository::new(unused_connection())),
            video_repository.clone(),
            Arc::new(SqliteChannelVideoRepository::new(unused_connection())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            Arc::new(SqliteEventPublisher::new(
                unused_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(MetadataGenerator::new(
                    Arc::new(FakeYoutubeMetadataRepository::default()),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(SqliteVideoMetadataRepository::new(unused_connection())),
                task_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(ThumbnailFetcher::new(
                    video_repository,
                    Arc::new(FakeVideoDownloaderRepository::default()),
                    task_repository.clone(),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
        ))
    }

    fn unused_connection() -> crate::infrastructure::shared::sqlite_connection::Database {
        crate::infrastructure::shared::sqlite_connection::Database::single(
            Connection::open_in_memory().unwrap(),
        )
    }

    /// Saves a video and its `@somechannel` membership at position 0,
    /// returning the membership as stored (with its storage-assigned `id`).
    fn save_channel_video(
        video_repository: &dyn VideoRepository,
        channel_video_repository: &dyn ChannelVideoRepository,
        video: &Video,
    ) -> ChannelVideo {
        save_channel_video_at(video_repository, channel_video_repository, video, 0)
    }

    /// Like `save_channel_video`, at the given recency position.
    fn save_channel_video_at(
        video_repository: &dyn VideoRepository,
        channel_video_repository: &dyn ChannelVideoRepository,
        video: &Video,
        position: i64,
    ) -> ChannelVideo {
        video_repository.save(video).unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                handle("@somechannel"),
                video.id.clone(),
                position,
                fixed_timestamp(),
            ))
            .unwrap();
        channel_video_repository
            .find_by_video(&video.id)
            .unwrap()
            .unwrap()
    }

    fn channel(channel_handle: &str) -> Channel {
        Channel::create(
            handle(channel_handle),
            "Some Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    fn watched_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_800_000_000, 0).unwrap()
    }

    fn my_video() -> Video {
        Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp())
    }

    fn downloaded_video(filename: &str, thumbnail_filename: Option<&str>) -> Video {
        my_video()
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                filename,
                thumbnail_filename.map(str::to_string),
                None,
                fixed_timestamp(),
            )
    }

    fn listed_video(youtube_id: &str, title: &str, position: i64) -> ChannelVideoListing {
        ChannelVideoListing {
            youtube_id: youtube_id.to_string(),
            title: title.to_string(),
            position,
        }
    }

    fn youtube_metadata(title: &str) -> YoutubeMetadata {
        YoutubeMetadata {
            title: title.to_string(),
            description: "A description".to_string(),
            channel_title: "My Channel".to_string(),
            published_at: fixed_timestamp(),
            tags: Vec::new(),
            category_id: None,
        }
    }

    /// The thumbnail the fake downloader reports for `my_video()`.
    fn fetched_thumbnail() -> FetchedThumbnail {
        FetchedThumbnail {
            folder: "My Video".to_string(),
            filename: "My Video.jpg".to_string(),
        }
    }

    fn next_reconcile(id: i64) -> ScheduledTask {
        pending_task(
            id,
            &Task::ReconcileChannel {
                channel_id: "@somechannel".to_string(),
            },
            fixed_timestamp() + chrono::Duration::seconds(3600),
        )
    }

    fn download_video_task(id: i64, video_id: &VideoRecordId) -> ScheduledTask {
        pending_task(
            id,
            &Task::DownloadVideo {
                video_id: video_id.as_str().to_string(),
                quality: "high".to_string(),
                output_dir: "/videos/creators/somechannel".to_string(),
            },
            fixed_timestamp(),
        )
    }

    fn fetch_thumbnail_task(id: i64, video_id: &VideoRecordId) -> ScheduledTask {
        pending_task(
            id,
            &Task::FetchThumbnail {
                video_id: video_id.as_str().to_string(),
                output_dir: "/videos/creators/somechannel".to_string(),
            },
            fixed_timestamp(),
        )
    }

    fn pending_task(id: i64, task: &Task, run_at: DateTime<Utc>) -> ScheduledTask {
        ScheduledTask {
            id,
            task_type: task.task_type().to_string(),
            payload: task.payload().to_string(),
            status: TaskStatus::Pending,
            retries: 0,
            run_at,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
    }

    /// The outbox row `SqliteEventPublisher` writes for `event`, as read back
    /// before the consumer has dispatched it.
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

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn handle(value: &str) -> ChannelHandle {
        ChannelHandle::new(value).unwrap()
    }

    fn payload_for(channel_id: &str) -> String {
        Task::ReconcileChannel {
            channel_id: channel_id.to_string(),
        }
        .payload()
        .to_string()
    }

    fn run(task: &ReconcileChannelTask, payload: &str) -> Result<(), String> {
        task.handle(payload, false).map_err(|e| e.to_string())
    }
}
