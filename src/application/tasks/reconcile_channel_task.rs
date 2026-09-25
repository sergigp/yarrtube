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
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::services::ThumbnailFetcher;
    use crate::domain::shared::Quality;
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::domain::video::Video;
    use crate::domain::video::VideoId;
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
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    #[test]
    fn it_should_skip_if_channel_is_gone() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
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
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
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
                DomainEvent::VideoAddedToChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: video_id.as_str().to_string(),
                }
            )]
        );
    }

    #[test]
    fn it_should_refresh_existing_videos() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
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
    fn it_should_evict_videos_beyond_the_limit() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
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
                DomainEvent::VideoRemovedFromChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: existing.id.as_str().to_string(),
                    title: "Old".to_string(),
                    filename: None,
                    thumbnail_filename: None,
                    was_downloaded: false,
                }
            )]
        );
    }

    #[test]
    fn it_should_keep_videos_if_listing_fails() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
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

        assert_eq!(
            result,
            Err("yt-dlp failed to list channel videos".to_string())
        );
        assert_eq!(video_repository.list().unwrap(), vec![existing]);
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            vec![existing_channel_video]
        );
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_always_schedule_the_next_reconcile() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
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
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_video("My Video.mp4", Some("My Video.jpg"));
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
            vec![video.clone().reset_for_redownload(fixed_timestamp())]
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
    fn it_should_redownload_videos_with_non_mp4_file() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.webm".to_string(),
        ]));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_video("My Video.webm", None);
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
            vec![video.clone().reset_for_redownload(fixed_timestamp())]
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
    fn it_should_delete_orphaned_files() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "orphan.mp4".to_string(),
        ]));
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
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("orphan.mp4")]
        );
    }

    #[test]
    fn it_should_retry_permanently_errored_videos() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = my_video()
            .start_download(fixed_timestamp())
            .mark_errored(fixed_timestamp());
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
            vec![video.clone().reset_for_redownload(fixed_timestamp())]
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
    fn it_should_keep_matching_files() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.mp4".to_string(),
        ]));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_video("My Video.mp4", None);
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
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video]);
    }

    #[test]
    fn it_should_keep_matching_thumbnails() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.mp4".to_string(),
            "My Video.jpg".to_string(),
        ]));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_video("My Video.mp4", Some("My Video.jpg"));
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
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video]);
    }

    #[test]
    fn it_should_delete_stray_thumbnails() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.mp4".to_string(),
            "stray.jpg".to_string(),
        ]));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_video("My Video.mp4", None);
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
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("stray.jpg")]
        );
    }

    #[test]
    fn it_should_keep_videos_stored_in_their_own_folder() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository {
            list_result: Mutex::new(Some(Ok(vec!["My Video".to_string()]))),
            file_exists_result: Mutex::new(Some(true)),
            ..Default::default()
        });
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_video("My Video/My Video.mp4", None);
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
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![next_reconcile(1)]
        );
    }

    #[test]
    fn it_should_keep_thumbnails_of_pending_videos() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video".to_string(),
        ]));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = my_video().with_thumbnail("My Video/My Video.jpg", fixed_timestamp());
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
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video]);
    }

    #[test]
    fn it_should_generate_missing_metadata() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let video_metadata_repository =
            Arc::new(SqliteVideoMetadataRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
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
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata("My Video")),
            }),
            video_metadata_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.shared_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                Arc::new(FakeVideoDownloaderRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            videos_root.path().to_string_lossy(),
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
                "2023-11-14",
                2023,
                None,
                Vec::new(),
                "yt1",
                None,
                "20231114 My Video",
            ))
        );
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![next_reconcile(1)]
        );
    }

    #[test]
    fn it_should_keep_existing_metadata() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let video_metadata_repository =
            Arc::new(SqliteVideoMetadataRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let videos_root = tempfile::tempdir().unwrap();
        let video_dir = videos_root.path().join("creators/somechannel/My Video");
        std::fs::create_dir_all(&video_dir).unwrap();
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_video("My Video/My Video.mp4", None);
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &video,
        );
        let existing_metadata = VideoMetadata::new(
            "Stale Title",
            "Stale plot",
            "Stale Channel",
            "Stale Channel",
            "2020-01-01",
            2020,
            None,
            Vec::new(),
            "yt1",
            None,
            "0000 Stale Title",
        );
        video_metadata_repository
            .save(&video.id, &existing_metadata, &video_dir)
            .unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository,
            video_repository.clone(),
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "My Video", 0),
            ])),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata("Fresh Title")),
            }),
            video_metadata_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.shared_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository,
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                Arc::new(FakeVideoDownloaderRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            videos_root.path().to_string_lossy(),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(existing_metadata)
        );
    }

    #[test]
    fn it_should_fetch_thumbnails_of_new_videos() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
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
                listed_video("yt1", "My Video", 0),
            ])),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
            Arc::new(SqliteEventPublisher::new(
                db.shared_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository,
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        let videos = video_repository.list().unwrap();
        let video_id = videos[0].id.clone();
        assert_eq!(
            videos,
            vec![Video {
                id: video_id.clone(),
                ..my_video().with_thumbnail("My Video/My Video.jpg", fixed_timestamp())
            }]
        );
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![thumbnail_call(None)]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoAddedToChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: video_id.as_str().to_string(),
                }
            )]
        );
    }

    #[test]
    fn it_should_add_new_videos_if_thumbnail_fetch_fails() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let video_downloader_repository =
            Arc::new(FakeVideoDownloaderRepository::default().with_thumbnail_error());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository,
            video_repository.clone(),
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "My Video", 0),
            ])),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
            Arc::new(SqliteEventPublisher::new(
                db.shared_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository,
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        let videos = video_repository.list().unwrap();
        let video_id = videos[0].id.clone();
        assert_eq!(
            videos,
            vec![Video {
                id: video_id.clone(),
                ..my_video()
            }]
        );
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![thumbnail_call(None), thumbnail_call(None)]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoAddedToChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: video_id.as_str().to_string(),
                }
            )]
        );
    }

    #[test]
    fn it_should_fetch_missing_thumbnails() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_thumbnail_result(Some(fetched_thumbnail())),
        );
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = my_video();
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
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
            Arc::new(SqliteEventPublisher::new(
                db.shared_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository,
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.with_thumbnail("My Video/My Video.jpg", fixed_timestamp())]
        );
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![thumbnail_call(None)]
        );
    }

    #[test]
    fn it_should_fetch_missing_thumbnails_into_the_video_folder() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_thumbnail_result(Some(fetched_thumbnail())),
        );
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
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
            Arc::new(SqliteEventPublisher::new(
                db.shared_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository,
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.with_thumbnail("My Video/My Video.jpg", fixed_timestamp())]
        );
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![thumbnail_call(Some("My Video"))]
        );
    }

    #[test]
    fn it_should_not_refetch_existing_thumbnails() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_downloader_repository = Arc::new(FakeVideoDownloaderRepository::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_video("My Video/My Video.mp4", Some("My Video/My Video.jpg"));
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
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
            Arc::new(SqliteEventPublisher::new(
                db.shared_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository,
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![]
        );
        assert_eq!(video_repository.list().unwrap(), vec![video]);
    }

    #[test]
    fn it_should_not_fetch_thumbnails_of_videos_being_redownloaded() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_thumbnail_result(Some(fetched_thumbnail())),
        );
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
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
            Arc::new(SqliteEventPublisher::new(
                db.shared_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(Vec::new())),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![]
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().reset_for_redownload(fixed_timestamp())]
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
    fn it_should_not_fetch_thumbnails_of_videos_downloading() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_thumbnail_result(Some(fetched_thumbnail())),
        );
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = my_video().start_download(fixed_timestamp());
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
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
            Arc::new(SqliteEventPublisher::new(
                db.shared_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository,
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(ThumbnailFetcher::new(
                video_repository.clone(),
                video_downloader_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_downloader_repository.thumbnail_calls.lock().unwrap(),
            vec![]
        );
        assert_eq!(video_repository.list().unwrap(), vec![video]);
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
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        ChannelVideoReconciler::new(
            channel_repository,
            video_repository,
            channel_video_repository,
            channel_videos_repository,
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
            Arc::new(SqliteEventPublisher::new(
                db.shared_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository,
            video_file_repository,
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        )
    }

    /// A task for tests whose payload is rejected before reaching the
    /// reconciler. Its repositories sit on an unmigrated in-memory database,
    /// so a payload that wrongly got through would fail loudly instead of
    /// passing.
    fn any_task() -> ReconcileChannelTask {
        let video_repository = Arc::new(SqliteVideoRepository::new(unused_connection()));
        ReconcileChannelTask::new(ChannelVideoReconciler::new(
            Arc::new(SqliteChannelRepository::new(unused_connection())),
            video_repository.clone(),
            Arc::new(SqliteChannelVideoRepository::new(unused_connection())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(unused_connection())),
            Arc::new(SqliteEventPublisher::new(
                Arc::new(Mutex::new(unused_connection())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteTaskRepository::new(
                Arc::new(Mutex::new(unused_connection())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(ThumbnailFetcher::new(
                video_repository,
                Arc::new(FakeVideoDownloaderRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        ))
    }

    fn unused_connection() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    /// Saves a video and its `@somechannel` membership at position 0,
    /// returning the membership as stored (with its storage-assigned `id`).
    fn save_channel_video(
        video_repository: &dyn VideoRepository,
        channel_video_repository: &dyn ChannelVideoRepository,
        video: &Video,
    ) -> ChannelVideo {
        video_repository.save(video).unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                handle("@somechannel"),
                video.id.clone(),
                0,
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

    /// One `fetch_thumbnail` call as `FakeVideoDownloaderRepository` records
    /// it, for `my_video()` in `@somechannel`'s output directory.
    fn thumbnail_call(
        existing_folder: Option<&str>,
    ) -> (String, String, String, PathBuf, Option<String>) {
        (
            "https://www.youtube.com/watch?v=yt1".to_string(),
            "My Video".to_string(),
            "yt1".to_string(),
            PathBuf::from("/videos/creators/somechannel"),
            existing_folder.map(str::to_string),
        )
    }

    /// The `(output_dir, entry)` pair the fake records for one delete call.
    fn deleted(entry: &str) -> (PathBuf, String) {
        (
            PathBuf::from("/videos/creators/somechannel"),
            entry.to_string(),
        )
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
