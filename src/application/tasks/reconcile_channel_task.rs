use crate::domain::channel::ChannelHandle;
use crate::domain::services::{ChannelVideoReconciler, ChannelVideoReconcilerApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};

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
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::ReconcileChannel { channel_id } = task else {
            return Err(unexpected_task(task));
        };
        let Ok(channel_id) = ChannelHandle::new(channel_id.clone()) else {
            return Ok(TaskOutcome::Done);
        };
        self.channel_video_reconciler
            .reconcile(channel_id)
            .map(|()| TaskOutcome::Done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tasks::log_capture::captured_info_lines;
    use crate::domain::category::{
        Category, CategoryColor, CategoryId, CategoryMember, CategoryName,
    };
    use crate::domain::channel::{Channel, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::channel_video::{VideoAddedToChannel, VideoRemovedFromChannel};
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::video::Chapters;
    use crate::domain::video::DownloadedQuality;

    use crate::domain::playlist::PlaylistPath;
    use crate::domain::preferences::SummaryModel;

    use crate::domain::services::ChannelAvatarRestorer;
    use crate::domain::services::InternalVideoReconciler;

    use crate::domain::services::ShowFileWriter;

    use crate::domain::services::VideoCategoryAssigner;

    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::show::ShowKey;
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::domain::video::Video;
    use crate::domain::video::{EpisodeNumber, FetchedThumbnail, VideoId, YoutubeVideoId};
    use crate::domain::video_category::{
        Verdict, VerdictAuthor, VideoCategories, VideoCategoryVerdict,
    };
    use crate::domain::video_summary::{SummaryText, VideoSummary};
    use crate::domain::video_transcript::{
        CaptionLanguage, CaptionSource, CaptionTrack, TranscriptText, VideoTranscript,
    };
    use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FakeChannelAvatarRepository;
    use crate::infrastructure::repositories::filesystem_show_file_repository::FakeShowFileRepository;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;

    use crate::infrastructure::repositories::sqlite_category_repository::{
        CategoryRepository, SqliteCategoryRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, SqliteChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, SqliteChannelVideoRepository,
    };

    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_categories_repository::{
        SqliteVideoCategoriesRepository, VideoCategoriesRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::{
        SqliteVideoCategoryVerdictRepository, VideoCategoryVerdictRepository,
    };

    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_summary_repository::{
        SqliteVideoSummaryRepository, VideoSummaryRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_transcript_repository::{
        SqliteVideoTranscriptRepository, VideoTranscriptRepository,
    };
    use crate::infrastructure::repositories::youtube_channel_repository::{
        FakeYoutubeChannelRepository, ResolvedChannel, YoutubeChannelRepository,
    };
    use crate::infrastructure::repositories::youtube_channel_videos_repository::{
        ChannelVideoListing, FakeChannelVideosRepository,
    };

    use crate::infrastructure::repositories::youtube_thumbnail_repository::FakeThumbnailRepository;
    use crate::infrastructure::shared::category_lock::CategoryLock;
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::show_locks::ShowLocks;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;

    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::path::PathBuf;
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
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
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

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
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
    fn it_should_log_a_pass_summary() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository {
            delete_result: std::sync::Mutex::new(Some(Ok(true))),
            ..FakeVideoFileRepository::with_listing(vec!["stray.mkv".to_string()])
        });
        channel_repository.insert(&channel("@somechannel")).unwrap();
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &Video::create(
                YoutubeVideoId::new("yt_old").unwrap(),
                "Old",
                fixed_timestamp(),
            ),
        );
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository,
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt_new", "New", 0),
            ])),
            task_repository,
            video_file_repository,
        ));

        let mut result = Err(String::new());
        let logs = captured_info_lines(|| result = run(&task, &task_for("@somechannel")));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            pass_lines(&logs),
            vec![
                "INFO reconciling channel channel_id=@somechannel",
                "INFO reconciled channel channel_id=@somechannel listed=1 added=1 removed=1 syncs_requested=1 orphans_deleted=1 listing_seconds=0.0 duration_seconds=0.0",
            ]
        );
    }

    #[test]
    fn it_should_log_a_pass_summary_if_listing_fails() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &my_video(),
        );
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository,
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::failing()),
            task_repository,
            Arc::new(FakeVideoFileRepository::default()),
        ));

        let mut result = Err(String::new());
        let logs = captured_info_lines(|| result = run(&task, &task_for("@somechannel")));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            pass_lines(&logs),
            vec![
                "INFO reconciling channel channel_id=@somechannel",
                "INFO reconciled channel channel_id=@somechannel added=0 removed=0 syncs_requested=1 orphans_deleted=0 listing_seconds=0.0 duration_seconds=0.0",
            ]
        );
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
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
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

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        let videos = video_repository.list().unwrap();
        let video_id = videos[0].id.clone();
        assert_eq!(
            videos,
            vec![Video {
                id: video_id.clone(),
                ..Video::create(
                    YoutubeVideoId::new("yt1").unwrap(),
                    "One",
                    fixed_timestamp()
                )
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
            vec![sync_video_task(1, &video_id), next_reconcile(2)]
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
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let existing = Video::create(
            YoutubeVideoId::new("yt1").unwrap(),
            "Original",
            fixed_timestamp(),
        );
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

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
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
    fn it_should_keep_the_yes_verdict_categories_of_an_uncategorized_channel() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let video_categories_repository =
            Arc::new(SqliteVideoCategoriesRepository::new(db.database()));
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let existing = my_video();
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &existing,
        );
        let chess = category_with_members("Chess", &[]);
        SqliteCategoryRepository::new(db.database())
            .insert(&chess)
            .unwrap();
        SqliteVideoCategoryVerdictRepository::new(db.database())
            .save(&VideoCategoryVerdict {
                video_id: existing.id.clone(),
                category_id: chess.id.clone(),
                verdict: Verdict::Yes,
                decided_by: VerdictAuthor::Ai,
                model: Some("qwen3:4b".to_string()),
                reason: Some("A chess game.".to_string()),
                decided_at: fixed_timestamp(),
            })
            .unwrap();
        let classified = VideoCategories::new(existing.id.clone(), vec![chess.id.clone()]);
        video_categories_repository
            .save_many(std::slice::from_ref(&classified))
            .unwrap();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_publisher = Arc::new(SqliteEventPublisher::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let _video_category_assigner = VideoCategoryAssigner::new(
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
            video_categories_repository.clone(),
            Arc::new(SqliteCategoryRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            event_publisher.clone(),
            Arc::new(CategoryLock::default()),
        );
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone().clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "My Video", 0),
            ])),
            event_publisher,
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone().clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository.clone(),
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            video_categories_repository.clone(),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_categories_repository
                .find_many(std::slice::from_ref(&existing.id))
                .unwrap(),
            vec![classified]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_delete_the_categories_of_a_removed_video() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let video_categories_repository =
            Arc::new(SqliteVideoCategoriesRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let removed = my_video();
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &removed,
        );
        video_categories_repository
            .save_many(&[VideoCategories::new(
                removed.id.clone(),
                vec![CategoryId::new("classified").unwrap()],
            )])
            .unwrap();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_publisher = Arc::new(SqliteEventPublisher::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let _video_category_assigner = VideoCategoryAssigner::new(
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
            video_categories_repository.clone(),
            Arc::new(SqliteCategoryRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            event_publisher.clone(),
            Arc::new(CategoryLock::default()),
        );
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone().clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::with_videos(vec![])),
            event_publisher,
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone().clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository.clone(),
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            video_categories_repository.clone(),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            video_categories_repository
                .find_many(std::slice::from_ref(&removed.id))
                .unwrap(),
            vec![VideoCategories::none(removed.id.clone())]
        );
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
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        channel_repository
            .insert(&Channel {
                video_limit: VideoLimit::new(1).unwrap(),
                ..channel("@somechannel")
            })
            .unwrap();
        let existing = Video::create(
            YoutubeVideoId::new("yt_old").unwrap(),
            "Old",
            fixed_timestamp(),
        );
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

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
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
                    subtitle_filename: None,
                    was_downloaded: false,
                })
            )]
        );
    }

    #[test]
    fn it_should_delete_the_transcript_of_a_removed_video() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_episode();
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &video,
        );
        video_transcript_repository
            .save(&VideoTranscript::create(
                video.id.clone(),
                &CaptionTrack {
                    key: "en".to_string(),
                    language: CaptionLanguage::new("en").unwrap(),
                    source: CaptionSource::Manual,
                },
                TranscriptText::new("Hello there"),
                fixed_timestamp(),
            ))
            .unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            video_transcript_repository.clone(),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_delete_the_summary_of_a_video_removed_from_the_channel() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_episode();
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &video,
        );
        video_transcript_repository
            .save(&VideoTranscript::create(
                video.id.clone(),
                &CaptionTrack {
                    key: "en".to_string(),
                    language: CaptionLanguage::new("en").unwrap(),
                    source: CaptionSource::Manual,
                },
                TranscriptText::new("Hello there"),
                fixed_timestamp(),
            ))
            .unwrap();
        let video_summary_repository = Arc::new(SqliteVideoSummaryRepository::new(db.database()));
        video_summary_repository
            .save(&VideoSummary::create(
                video.id.clone(),
                SummaryText::new("Three games."),
                &SummaryModel::default(),
                fixed_timestamp(),
            ))
            .unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            video_transcript_repository.clone(),
            video_summary_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(video_summary_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_delete_a_removed_videos_verdicts() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_episode();
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &video,
        );
        let verdict_repository = Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database()));
        verdict_repository
            .save(&VideoCategoryVerdict {
                video_id: video.id.clone(),
                category_id: CategoryId::new("sicilian").unwrap(),
                verdict: Verdict::Yes,
                decided_by: VerdictAuthor::Ai,
                model: Some("qwen3:4b".to_string()),
                reason: Some("A Sicilian game.".to_string()),
                decided_at: fixed_timestamp(),
            })
            .unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            video_transcript_repository.clone(),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            verdict_repository.clone(),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            verdict_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![]
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
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let original = Channel {
            video_limit: VideoLimit::new(3).unwrap(),
            ..channel("@somechannel")
        };
        channel_repository.insert(&original).unwrap();
        let newest = Video::create(
            YoutubeVideoId::new("yt1").unwrap(),
            "One",
            fixed_timestamp(),
        );
        let middle = Video::create(
            YoutubeVideoId::new("yt2").unwrap(),
            "Two",
            fixed_timestamp(),
        );
        let oldest = Video::create(
            YoutubeVideoId::new("yt3").unwrap(),
            "Three",
            fixed_timestamp(),
        );
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

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
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
                sync_video_task(1, &newest.id),
                sync_video_task(2, &middle.id),
                next_reconcile(3),
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
                    subtitle_filename: None,
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
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let original = Channel {
            video_limit: VideoLimit::new(1).unwrap(),
            ..channel("@somechannel")
        };
        channel_repository.insert(&original).unwrap();
        let newest = Video::create(
            YoutubeVideoId::new("yt1").unwrap(),
            "One",
            fixed_timestamp(),
        );
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

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
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
                    ..Video::create(
                        YoutubeVideoId::new("yt2").unwrap(),
                        "Two",
                        fixed_timestamp()
                    )
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
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                sync_video_task(1, &newest.id),
                sync_video_task(2, &added_id),
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
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let existing = Video::create(
            YoutubeVideoId::new("yt_old").unwrap(),
            "Old",
            fixed_timestamp(),
        );
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

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![existing.clone()]);
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            vec![existing_channel_video]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![sync_video_task(1, &existing.id), next_reconcile(2)]
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

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![next_reconcile(1)]
        );
    }

    #[test]
    fn it_should_schedule_the_next_reconcile_after_the_channels_interval() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        channel_repository
            .insert(
                &channel("@somechannel")
                    .with_reconcile_interval(ReconcileInterval::new(30).unwrap()),
            )
            .unwrap();
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::ReconcileChannel {
                    channel_id: "@somechannel".to_string(),
                },
                fixed_timestamp() + chrono::Duration::minutes(30),
            )]
        );
    }

    #[test]
    fn it_should_not_schedule_the_next_reconcile_if_one_is_already_pending() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let pending_reconcile = Task::ReconcileChannel {
            channel_id: "@somechannel".to_string(),
        };
        task_repository
            .schedule(
                &pending_reconcile,
                fixed_timestamp() + chrono::Duration::minutes(5),
            )
            .unwrap();
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &pending_reconcile,
                fixed_timestamp() + chrono::Duration::minutes(5),
            )]
        );
    }

    #[test]
    fn it_should_not_schedule_the_next_reconcile_if_channel_deleted_during_the_pass() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository.clone(),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::deleting_the_channel_from(
                channel_repository.clone(),
            )),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(channel_repository.list().unwrap(), vec![]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_rewrite_missing_show_files_on_a_recurring_reconcile() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let show_file_repository = Arc::new(FakeShowFileRepository::default());
        channel_repository
            .insert(&Channel {
                avatar_filename: Some("@somechannel.jpg".to_string()),
                ..channel("@somechannel")
            })
            .unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                show_file_repository.clone(),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            *show_file_repository.show_nfos.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/creators/somechannel"),
                concat!(
                    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                    "<tvshow><title>Some Channel</title><studio>Some Channel</studio>",
                    r#"<uniqueid type="youtube">UC123</uniqueid><status>Continuing</status></tvshow>"#,
                )
                .to_string()
            )]
        );
        assert_eq!(
            *show_file_repository.posters.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/creators/somechannel"),
                PathBuf::from("avatars/@somechannel.jpg")
            )]
        );
    }

    #[test]
    fn it_should_wait_for_a_move_before_reconciling() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let show_file_repository = Arc::new(FakeShowFileRepository::default());
        let show_locks = Arc::new(ShowLocks::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                show_file_repository.clone(),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository.clone(),
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            show_locks.clone(),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        ));

        let result = std::thread::scope(|scope| {
            let moving = show_locks.lock(ShowKey::Channel(handle("@somechannel")));
            let reconciling = scope.spawn(|| run(&task, &task_for("@somechannel")));
            // Gives a reconcile that doesn't wait every chance to run first.
            std::thread::sleep(std::time::Duration::from_millis(100));
            channel_repository
                .update(
                    &channel("@somechannel")
                        .with_path(PlaylistPath::new("science/somechannel").unwrap()),
                )
                .unwrap();
            drop(moving);
            reconciling.join().unwrap()
        });

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            *show_file_repository.show_nfos.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/science/somechannel"),
                concat!(
                    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                    "<tvshow><title>Some Channel</title><studio>Some Channel</studio>",
                    r#"<uniqueid type="youtube">UC123</uniqueid><status>Continuing</status></tvshow>"#,
                )
                .to_string()
            )]
        );
    }

    #[test]
    fn it_should_restore_a_missing_channel_avatar() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let show_file_repository = Arc::new(FakeShowFileRepository::default());
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::default());
        let stored_channel = Channel {
            avatar_filename: Some("@somechannel.jpg".to_string()),
            ..channel("@somechannel")
        };
        channel_repository.insert(&stored_channel).unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                show_file_repository.clone(),
                avatar_repository.clone(),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository.clone(),
                Arc::new(FakeYoutubeChannelRepository {
                    resolved: Some(resolved_channel(Some("https://yt3.example/avatar.jpg"))),
                }),
                avatar_repository.clone(),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            avatar_repository.avatars(),
            vec![(
                "@somechannel.jpg".to_string(),
                "https://yt3.example/avatar.jpg".to_string()
            )]
        );
        assert_eq!(
            *show_file_repository.posters.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/creators/somechannel"),
                PathBuf::from("avatars/@somechannel.jpg")
            )]
        );
        assert_eq!(channel_repository.list().unwrap(), vec![stored_channel]);
    }

    #[test]
    fn it_should_keep_a_stored_channel_avatar() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let show_file_repository = Arc::new(FakeShowFileRepository::default());
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::with_avatars(&[(
            "@somechannel.jpg",
            "https://yt3.example/original.jpg",
        )]));
        let stored_channel = Channel {
            avatar_filename: Some("@somechannel.jpg".to_string()),
            ..channel("@somechannel")
        };
        channel_repository.insert(&stored_channel).unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                show_file_repository.clone(),
                avatar_repository.clone(),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository.clone(),
                Arc::new(FakeYoutubeChannelRepository {
                    resolved: Some(resolved_channel(Some("https://yt3.example/avatar.jpg"))),
                }),
                avatar_repository.clone(),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            avatar_repository.avatars(),
            vec![(
                "@somechannel.jpg".to_string(),
                "https://yt3.example/original.jpg".to_string()
            )]
        );
        assert_eq!(
            *show_file_repository.posters.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/creators/somechannel"),
                PathBuf::from("avatars/@somechannel.jpg")
            )]
        );
        assert_eq!(channel_repository.list().unwrap(), vec![stored_channel]);
    }

    #[test]
    fn it_should_record_a_restored_avatar_for_a_channel_without_one() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let show_file_repository = Arc::new(FakeShowFileRepository::default());
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                show_file_repository.clone(),
                avatar_repository.clone(),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository.clone(),
                Arc::new(FakeYoutubeChannelRepository {
                    resolved: Some(resolved_channel(Some("https://yt3.example/avatar.jpg"))),
                }),
                avatar_repository.clone(),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            avatar_repository.avatars(),
            vec![(
                "@somechannel.jpg".to_string(),
                "https://yt3.example/avatar.jpg".to_string()
            )]
        );
        assert_eq!(
            *show_file_repository.posters.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/creators/somechannel"),
                PathBuf::from("avatars/@somechannel.jpg")
            )]
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![Channel {
                avatar_filename: Some("@somechannel.jpg".to_string()),
                ..channel("@somechannel")
            }]
        );
    }

    #[test]
    fn it_should_keep_settings_changed_while_restoring_a_channel_avatar() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let show_file_repository = Arc::new(FakeShowFileRepository::default());
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                show_file_repository.clone(),
                avatar_repository.clone(),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository.clone(),
                Arc::new(SettingsEditingYoutubeChannelRepository {
                    channel_repository: channel_repository.clone(),
                    edited: Channel {
                        video_limit: VideoLimit::new(50).unwrap(),
                        ..channel("@somechannel")
                    },
                    resolved: resolved_channel(Some("https://yt3.example/avatar.jpg")),
                }),
                avatar_repository.clone(),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            avatar_repository.avatars(),
            vec![(
                "@somechannel.jpg".to_string(),
                "https://yt3.example/avatar.jpg".to_string()
            )]
        );
        assert_eq!(
            *show_file_repository.posters.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/creators/somechannel"),
                PathBuf::from("avatars/@somechannel.jpg")
            )]
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![Channel {
                video_limit: VideoLimit::new(50).unwrap(),
                avatar_filename: Some("@somechannel.jpg".to_string()),
                ..channel("@somechannel")
            }]
        );
    }

    #[test]
    fn it_should_sync_the_channel_even_if_its_avatar_cannot_be_restored() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let show_file_repository = Arc::new(FakeShowFileRepository::default());
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::unavailable());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "One", 0),
            ])),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                show_file_repository.clone(),
                avatar_repository.clone(),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository.clone(),
                Arc::new(FakeYoutubeChannelRepository {
                    resolved: Some(resolved_channel(Some("https://yt3.example/avatar.jpg"))),
                }),
                avatar_repository.clone(),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(avatar_repository.avatars(), vec![]);
        assert_eq!(*show_file_repository.posters.lock().unwrap(), vec![]);
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        let videos = video_repository.list().unwrap();
        let video_id = videos[0].id.clone();
        assert_eq!(
            videos,
            vec![Video {
                id: video_id.clone(),
                ..Video::create(
                    YoutubeVideoId::new("yt1").unwrap(),
                    "One",
                    fixed_timestamp()
                )
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
            vec![sync_video_task(1, &video_id), next_reconcile(2)]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoAddedToChannel(VideoAddedToChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: videos[0].id.as_str().to_string(),
                })
            )]
        );
    }

    #[test]
    fn it_should_sync_the_channel_even_if_its_show_files_cannot_be_written() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
        let show_file_repository = Arc::new(FakeShowFileRepository::failing());
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "One", 0),
            ])),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                show_file_repository.clone(),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        let videos = video_repository.list().unwrap();
        let video_id = videos[0].id.clone();
        assert_eq!(
            videos,
            vec![Video {
                id: video_id.clone(),
                ..Video::create(
                    YoutubeVideoId::new("yt1").unwrap(),
                    "One",
                    fixed_timestamp()
                )
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
        assert_eq!(*show_file_repository.show_nfos.lock().unwrap(), vec![]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![sync_video_task(1, &video_id), next_reconcile(2)]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoAddedToChannel(VideoAddedToChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: videos[0].id.as_str().to_string(),
                })
            )]
        );
    }

    #[test]
    fn it_should_skip_if_invalid_channel_id_provided() {
        let result = run(&any_task(), &task_for(""));

        assert_eq!(result, Ok(TaskOutcome::Done));
    }

    #[test]
    fn it_should_fail_if_handed_another_task() {
        let result = run(&any_task(), &Task::UpdateYtdlp);

        assert_eq!(
            result,
            Err("unexpected update_ytdlp task for this handler".to_string())
        );
    }

    #[test]
    fn it_should_schedule_a_sync_of_every_stored_video() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let first = my_video();
        let second = Video::create(
            YoutubeVideoId::new("yt2").unwrap(),
            "Other Video",
            fixed_timestamp(),
        );
        save_channel_video_at(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &first,
            0,
        );
        save_channel_video_at(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &second,
            1,
        );
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "My Video", 0),
                listed_video("yt2", "Other Video", 1),
            ])),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![first.clone(), second.clone()]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                sync_video_task(1, &first.id),
                sync_video_task(2, &second.id),
                next_reconcile(3),
            ]
        );
    }

    #[test]
    fn it_should_keep_recorded_subtitle_files() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(
            [
                episode_files(),
                vec![
                    "Season 2023/S2023E111401 - My Video.en.srt".to_string(),
                    "Season 2023/S2023E111401 - My Video.fr.srt".to_string(),
                ],
            ]
            .concat(),
        ));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_episode().mark_subtitles_fetched(
            "Season 2023/S2023E111401 - My Video.en.srt",
            fixed_timestamp(),
        );
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

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/creators/somechannel"),
                "Season 2023/S2023E111401 - My Video.fr.srt".to_string(),
            )]
        );
    }

    #[test]
    fn it_should_keep_subtitle_files_placed_but_not_yet_recorded() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(
            [
                episode_files(),
                vec!["Season 2023/S2023E111401 - My Video.en.srt".to_string()],
            ]
            .concat(),
        ));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_episode();
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

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_keep_unrecorded_subtitle_files_of_a_retitled_video() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(
            [
                episode_files(),
                vec!["Season 2023/S2023E111401 - My Video.en.srt".to_string()],
            ]
            .concat(),
        ));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = Video {
            title: "New Title".to_string(),
            ..downloaded_episode()
        };
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
                listed_video("yt1", "New Title", 0),
            ])),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
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
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
        let thumbnail_repository = Arc::new(FakeThumbnailRepository::with_result(Some(
            fetched_thumbnail(),
        )));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
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
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        let videos = video_repository.list().unwrap();
        let (first_id, second_id) = (videos[0].id.clone(), videos[1].id.clone());
        assert_eq!(
            videos,
            vec![
                Video {
                    id: first_id.clone(),
                    ..Video::create(
                        YoutubeVideoId::new("yt1").unwrap(),
                        "One",
                        fixed_timestamp()
                    )
                },
                Video {
                    id: second_id.clone(),
                    ..Video::create(
                        YoutubeVideoId::new("yt2").unwrap(),
                        "Two",
                        fixed_timestamp()
                    )
                },
            ]
        );
        assert_eq!(*thumbnail_repository.calls.lock().unwrap(), vec![]);
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
    fn it_should_leave_a_broken_video_to_its_sync() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
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
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "My Video", 0),
            ])),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(Vec::new())),
        ));

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![sync_video_task(1, &video.id), next_reconcile(2)]
        );
    }

    #[test]
    fn it_should_delete_the_stale_file_of_a_reset_video() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.mkv".to_string(),
        ]));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_video("My Video.mkv", None).reset_for_redownload(fixed_timestamp());
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

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/creators/somechannel"),
                "My Video.mkv".to_string()
            )]
        );
    }

    #[test]
    fn it_should_delete_the_stale_file_of_a_reset_episode() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "Season 2023".to_string(),
            "Season 2023/S2023E111401 - My Video.mkv".to_string(),
            "Season 2023/S2023E111401 - My Video.jpg".to_string(),
        ]));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = Video {
            episode_number: Some(recovered_episode()),
            ..downloaded_video(
                "Season 2023/S2023E111401 - My Video.mkv",
                Some("Season 2023/S2023E111401 - My Video.jpg"),
            )
        }
        .reset_for_redownload(fixed_timestamp());
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

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/creators/somechannel"),
                "Season 2023/S2023E111401 - My Video.mkv".to_string()
            )]
        );
    }

    #[test]
    fn it_should_schedule_the_sync_of_a_video_added_by_the_pass() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
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

        let result = run(&task, &task_for("@somechannel"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        let added = video_repository.list().unwrap()[0].id.clone();
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![sync_video_task(1, &added), next_reconcile(2)]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoAddedToChannel(VideoAddedToChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: added.as_str().to_string(),
                })
            )]
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
        ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteVideoSummaryRepository::new(db.database())),
            channel_video_repository,
            channel_videos_repository,
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                video_file_repository.clone(),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
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
            Arc::new(SqliteVideoTranscriptRepository::new(unused_connection())),
            Arc::new(SqliteVideoSummaryRepository::new(unused_connection())),
            Arc::new(SqliteChannelVideoRepository::new(unused_connection())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            Arc::new(SqliteEventPublisher::new(
                unused_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                task_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                Arc::new(SqliteChannelRepository::new(unused_connection())),
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
            Arc::new(SqliteVideoCategoriesRepository::new(unused_connection())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(
                unused_connection(),
            )),
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
        video_repository.insert(video).unwrap();
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
            ReconcileInterval::default_interval(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    /// Saves `edited` over the stored channel while resolving it, the way a
    /// settings change landing mid-restore would, then resolves to `resolved`.
    struct SettingsEditingYoutubeChannelRepository {
        channel_repository: Arc<SqliteChannelRepository>,
        edited: Channel,
        resolved: ResolvedChannel,
    }

    impl YoutubeChannelRepository for SettingsEditingYoutubeChannelRepository {
        fn resolve(&self, _handle: &ChannelHandle) -> anyhow::Result<Option<ResolvedChannel>> {
            self.channel_repository.update(&self.edited)?;
            Ok(Some(self.resolved.clone()))
        }
    }

    fn resolved_channel(avatar_url: Option<&str>) -> ResolvedChannel {
        ResolvedChannel {
            youtube_channel_id: "UC123".to_string(),
            title: "Some Channel".to_string(),
            avatar_url: avatar_url.map(str::to_string),
        }
    }

    fn my_video() -> Video {
        Video::create(
            YoutubeVideoId::new("yt1").unwrap(),
            "My Video",
            fixed_timestamp(),
        )
    }

    /// Its chapters and quality settled, so a sync schedules no chapters fetch or
    /// quality check.
    fn downloaded_video(filename: &str, thumbnail_filename: Option<&str>) -> Video {
        Video {
            chapters: Chapters {
                found: Some(vec![]),
                ..Chapters::done_at(None)
            },
            downloaded_quality: DownloadedQuality::done_at(None),
            ..my_video()
                .start_download(fixed_timestamp())
                .mark_downloaded(
                    Quality::High,
                    filename,
                    thumbnail_filename.map(str::to_string),
                    None,
                    fixed_timestamp(),
                )
                .with_chapters(vec![], fixed_timestamp())
        }
    }

    /// `my_video()` downloaded into its season folder as episode
    /// `2023`/`111401`, with its thumbnail, its subtitles still missing.
    fn downloaded_episode() -> Video {
        Video {
            episode_number: Some(recovered_episode()),
            ..downloaded_video(
                "Season 2023/S2023E111401 - My Video.mp4",
                Some("Season 2023/S2023E111401 - My Video.jpg"),
            )
        }
    }

    /// The files `downloaded_episode()` has on disk.
    fn episode_files() -> Vec<String> {
        vec![
            "Season 2023".to_string(),
            "Season 2023/S2023E111401 - My Video.mp4".to_string(),
            "Season 2023/S2023E111401 - My Video.jpg".to_string(),
            "Season 2023/S2023E111401 - My Video.nfo".to_string(),
        ]
    }

    /// The episode `my_video()` gets from `youtube_metadata()`'s publish time.
    fn recovered_episode() -> EpisodeNumber {
        EpisodeNumber::from_columns(2023, 111401).unwrap()
    }

    fn listed_video(youtube_id: &str, title: &str, position: i64) -> ChannelVideoListing {
        ChannelVideoListing {
            youtube_id: youtube_id.to_string(),
            title: title.to_string(),
            position,
        }
    }

    /// The thumbnail the fake downloader reports for `my_video()`.
    fn fetched_thumbnail() -> FetchedThumbnail {
        FetchedThumbnail {
            path: std::path::PathBuf::from("My Video.jpg"),
            published_at: None,
        }
    }

    fn next_reconcile(id: i64) -> ScheduledTask {
        pending_task(
            id,
            &Task::ReconcileChannel {
                channel_id: "@somechannel".to_string(),
            },
            fixed_timestamp() + chrono::Duration::minutes(180),
        )
    }

    fn sync_video_task(id: i64, video_id: &VideoId) -> ScheduledTask {
        pending_task(
            id,
            &Task::SyncVideo {
                video_id: video_id.as_str().to_string(),
            },
            fixed_timestamp(),
        )
    }

    fn pending_task(id: i64, task: &Task, run_at: DateTime<Utc>) -> ScheduledTask {
        ScheduledTask {
            id,
            task: task.clone(),
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
            run_at: fixed_timestamp(),
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
    }

    fn category_with_members(name: &str, members: &[CategoryMember]) -> Category {
        Category::create(
            CategoryName::new(name).unwrap(),
            CategoryColor::new(CategoryColor::PALETTE[0]).unwrap(),
            fixed_timestamp(),
        )
        .with_members(members.to_vec())
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn handle(value: &str) -> ChannelHandle {
        ChannelHandle::new(value).unwrap()
    }

    fn task_for(channel_id: &str) -> Task {
        Task::ReconcileChannel {
            channel_id: channel_id.to_string(),
        }
    }

    /// The pass's start and summary lines among captured lines.
    fn pass_lines(logs: &[String]) -> Vec<&str> {
        logs.iter()
            .map(String::as_str)
            .filter(|line| {
                line.starts_with("INFO reconciling channel ")
                    || line.starts_with("INFO reconciled channel ")
            })
            .collect()
    }

    fn run(task: &ReconcileChannelTask, queued: &Task) -> Result<TaskOutcome, String> {
        task.handle(queued).map_err(|e| e.to_string())
    }
}
