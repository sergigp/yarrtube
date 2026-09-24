use crate::domain::services::PlaylistVideoReconciler;
use crate::domain::shared::PlaylistId;
use crate::domain::task::Task;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Runs every subsequent reconcile for a playlist (the first one is
/// triggered by `subscribers::reconcile_on_playlist_created` instead).
pub struct ReconcilePlaylistTask {
    playlist_video_reconciler: PlaylistVideoReconciler,
}

impl ReconcilePlaylistTask {
    pub fn new(playlist_video_reconciler: PlaylistVideoReconciler) -> Self {
        Self {
            playlist_video_reconciler,
        }
    }
}

impl TaskHandler for ReconcilePlaylistTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let playlist_id = Task::decode_reconcile_playlist_payload(payload)?;
        let Ok(playlist_id) = PlaylistId::new(playlist_id) else {
            return Ok(());
        };
        self.playlist_video_reconciler.reconcile(playlist_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::services::ThumbnailFetcher;
    use crate::domain::shared::{Quality, VideoId};
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::domain::video::Video;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        PlaylistRepository, SqlitePlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::{
        PlaylistVideoRepository, SqlitePlaylistVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::SqliteVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::{
        FakeYoutubePlaylistItemsRepository, YoutubePlaylistItem,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    #[test]
    fn it_should_skip_if_playlist_is_gone() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            Vec::new(),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL404"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_if_invalid_playlist_id_provided() {
        let result = run(&any_task(), &payload_for(""));

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn it_should_reject_a_malformed_payload() {
        let result = run(&any_task(), "not json");

        assert_eq!(
            result,
            Err(
                "invalid reconcile_playlist payload: expected ident at line 1 column 2".to_string()
            )
        );
    }

    #[test]
    fn it_should_add_new_videos() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![playlist_item("vid1", "One", 0)],
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        let videos = video_repository.list().unwrap();
        let video_id = videos[0].id.clone();
        assert_eq!(
            videos,
            vec![Video {
                id: video_id.clone(),
                ..Video::create(VideoId::new("vid1").unwrap(), "One", fixed_timestamp())
            }]
        );
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id())
                .unwrap(),
            vec![PlaylistVideo {
                id: 1,
                ..PlaylistVideo::create_with_position(
                    playlist_id(),
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
                DomainEvent::VideoAddedToPlaylist {
                    playlist_id: "PL1".to_string(),
                    video_id: video_id.as_str().to_string(),
                }
            )]
        );
    }

    #[test]
    fn it_should_refresh_title_of_existing_videos() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let existing = Video::create(VideoId::new("vid1").unwrap(), "Original", fixed_timestamp())
            .start_download(fixed_timestamp());
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &existing,
            0,
        );
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![playlist_item("vid1", "Renamed", 0)],
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                title: "Renamed".to_string(),
                ..existing
            }]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_refresh_position_of_existing_videos() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let existing = Video::create(VideoId::new("vid1").unwrap(), "One", fixed_timestamp());
        let existing_playlist_video = save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &existing,
            0,
        );
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![playlist_item("vid1", "One", 2)],
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id())
                .unwrap(),
            vec![PlaylistVideo {
                position: Some(2),
                ..existing_playlist_video
            }]
        );
    }

    #[test]
    fn it_should_remove_videos_gone_from_youtube() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let _stale = Video::create(VideoId::new("vid1").unwrap(), "Stale", fixed_timestamp());
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &_stale,
            0,
        );
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            Vec::new(),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id())
                .unwrap(),
            vec![]
        );
    }

    #[test]
    fn it_should_flag_downloaded_state_on_video_removed_events() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let downloaded = Video::create(
            VideoId::new("vid1").unwrap(),
            "Downloaded Video",
            fixed_timestamp(),
        )
        .start_download(fixed_timestamp())
        .mark_downloaded(
            Quality::High,
            "Downloaded Video.mp4",
            Some("Downloaded Video.jpg".to_string()),
            None,
            fixed_timestamp(),
        );
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &downloaded,
            0,
        );
        let pending = Video::create(
            VideoId::new("vid2").unwrap(),
            "Pending Video",
            fixed_timestamp(),
        );
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &pending,
            1,
        );
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            Vec::new(),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![
                pending_event(
                    1,
                    DomainEvent::VideoRemovedFromPlaylist {
                        playlist_id: "PL1".to_string(),
                        video_id: downloaded.id.as_str().to_string(),
                        title: "Downloaded Video".to_string(),
                        filename: Some("Downloaded Video.mp4".to_string()),
                        thumbnail_filename: Some("Downloaded Video.jpg".to_string()),
                        was_downloaded: true,
                    }
                ),
                pending_event(
                    2,
                    DomainEvent::VideoRemovedFromPlaylist {
                        playlist_id: "PL1".to_string(),
                        video_id: pending.id.as_str().to_string(),
                        title: "Pending Video".to_string(),
                        filename: None,
                        thumbnail_filename: None,
                        was_downloaded: false,
                    }
                ),
            ]
        );
    }

    #[test]
    fn it_should_always_schedule_the_next_reconcile() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            Vec::new(),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![next_reconcile(1)]
        );
    }

    #[test]
    fn it_should_redownload_videos_with_missing_file() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = downloaded_video("My Video.mp4", Some("My Video.jpg"));
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &video,
            0,
        );
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![member_playlist_item()],
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

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
                        output_dir: "/videos/my-playlist".to_string(),
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
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.webm".to_string(),
        ]));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = downloaded_video("My Video.webm", None);
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &video,
            0,
        );
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![member_playlist_item()],
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

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
                        output_dir: "/videos/my-playlist".to_string(),
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
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "orphan.mp4".to_string(),
        ]));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            Vec::new(),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("orphan.mp4")]
        );
    }

    #[test]
    fn it_should_retry_permanently_errored_videos() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = my_video()
            .start_download(fixed_timestamp())
            .mark_errored(fixed_timestamp());
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &video,
            0,
        );
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![member_playlist_item()],
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

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
                        output_dir: "/videos/my-playlist".to_string(),
                    },
                    fixed_timestamp(),
                ),
                next_reconcile(2),
            ]
        );
    }

    #[test]
    fn it_should_retry_a_video_that_failed_again() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(Vec::new()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = my_video()
            .start_download(fixed_timestamp())
            .mark_errored(fixed_timestamp());
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &video,
            0,
        );
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![member_playlist_item()],
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        run(&task, &payload_for("PL1")).unwrap();
        video_repository
            .update(
                &video
                    .clone()
                    .reset_for_redownload(fixed_timestamp())
                    .start_download(fixed_timestamp())
                    .mark_errored(fixed_timestamp()),
            )
            .unwrap();
        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.reset_for_redownload(fixed_timestamp())]
        );
    }

    #[test]
    fn it_should_keep_matching_files() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.mp4".to_string(),
        ]));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = downloaded_video("My Video.mp4", None);
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &video,
            0,
        );
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![member_playlist_item()],
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video]);
    }

    #[test]
    fn it_should_keep_matching_thumbnails() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.mp4".to_string(),
            "My Video.jpg".to_string(),
        ]));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = downloaded_video("My Video.mp4", Some("My Video.jpg"));
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &video,
            0,
        );
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![member_playlist_item()],
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
        assert_eq!(video_repository.list().unwrap(), vec![video]);
    }

    #[test]
    fn it_should_delete_stray_thumbnails() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.mp4".to_string(),
            "stray.jpg".to_string(),
        ]));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let _video = downloaded_video("My Video.mp4", None);
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &_video,
            0,
        );
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![member_playlist_item()],
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![deleted("stray.jpg")]
        );
    }

    /// Builds a reconciler around the repositories and fakes a test seeds,
    /// configures or asserts; the remaining ports (YouTube metadata, video
    /// metadata, thumbnails) are ones no reconcile test here observes. Events
    /// go to `db`'s outbox table.
    fn playlist_video_reconciler(
        db: &TestDatabase,
        playlist_repository: Arc<SqlitePlaylistRepository>,
        video_repository: Arc<SqliteVideoRepository>,
        playlist_video_repository: Arc<SqlitePlaylistVideoRepository>,
        playlist_items: Vec<YoutubePlaylistItem>,
        task_repository: Arc<SqliteTaskRepository>,
        video_file_repository: Arc<FakeVideoFileRepository>,
    ) -> PlaylistVideoReconciler {
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        PlaylistVideoReconciler::new(
            playlist_repository,
            video_repository,
            playlist_video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository {
                videos: Mutex::new(playlist_items),
            }),
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
    fn any_task() -> ReconcilePlaylistTask {
        let video_repository = Arc::new(SqliteVideoRepository::new(unused_connection()));
        ReconcilePlaylistTask::new(PlaylistVideoReconciler::new(
            Arc::new(SqlitePlaylistRepository::new(unused_connection())),
            video_repository.clone(),
            Arc::new(SqlitePlaylistVideoRepository::new(unused_connection())),
            Arc::new(FakeYoutubePlaylistItemsRepository {
                videos: Mutex::new(Vec::new()),
            }),
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

    /// Saves a video and its `PL1` membership at `position`, returning the
    /// membership as stored (with its storage-assigned `id`).
    fn save_playlist_video(
        video_repository: &dyn VideoRepository,
        playlist_video_repository: &dyn PlaylistVideoRepository,
        video: &Video,
        position: i64,
    ) -> PlaylistVideo {
        video_repository.save(video).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create_with_position(
                playlist_id(),
                video.id.clone(),
                position,
                fixed_timestamp(),
            ))
            .unwrap();
        playlist_video_repository
            .find_by_video(&video.id)
            .unwrap()
            .unwrap()
    }

    fn playlist(id: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new("My Playlist").unwrap(),
            PlaylistPath::new("my-playlist").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            fixed_timestamp(),
        )
    }

    fn playlist_id() -> PlaylistId {
        PlaylistId::new("PL1").unwrap()
    }

    fn my_video() -> Video {
        Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp())
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

    fn playlist_item(video_id: &str, title: &str, position: i64) -> YoutubePlaylistItem {
        YoutubePlaylistItem {
            video_id: video_id.to_string(),
            title: title.to_string(),
            position,
        }
    }

    /// How `my_video()` looks on YouTube, so membership sync leaves a seeded
    /// `my_video()` member alone instead of treating it as removed.
    fn member_playlist_item() -> YoutubePlaylistItem {
        playlist_item("vid1", "My Video", 0)
    }

    /// The `(output_dir, entry)` pair the fake records for one delete call.
    fn deleted(entry: &str) -> (PathBuf, String) {
        (PathBuf::from("/videos/my-playlist"), entry.to_string())
    }

    fn next_reconcile(id: i64) -> ScheduledTask {
        pending_task(
            id,
            &Task::ReconcilePlaylist {
                playlist_id: "PL1".to_string(),
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

    fn payload_for(playlist_id: &str) -> String {
        Task::ReconcilePlaylist {
            playlist_id: playlist_id.to_string(),
        }
        .payload()
        .to_string()
    }

    fn run(task: &ReconcilePlaylistTask, payload: &str) -> Result<(), String> {
        task.handle(payload, false).map_err(|e| e.to_string())
    }
}
