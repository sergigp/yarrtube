use crate::domain::services::{VideoReconciler, VideoReconcilerApi};
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Syncs one stored video: schedules whichever of its steps its stored
/// state still needs. Scheduled by the `SyncVideoOn*` subscribers when a
/// video is added to a playlist or channel and each time one is downloaded.
pub struct SyncVideoTask {
    video_reconciler: VideoReconciler,
}

impl SyncVideoTask {
    pub fn new(video_reconciler: VideoReconciler) -> Self {
        Self { video_reconciler }
    }
}

impl TaskHandler for SyncVideoTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let video_id = VideoRecordId::new(Task::decode_sync_video_payload(payload)?)?;
        self.video_reconciler.reconcile(&video_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tasks::log_capture::captured_log_messages;
    use crate::domain::category::{Category, CategoryColor, CategoryMember, CategoryName};
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::playlist::{Playlist, PlaylistId, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::services::MetadataGenerator;
    use crate::domain::services::VideoCategoryAssigner;
    use crate::domain::services::VideoRelocator;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::domain::video::{EpisodeNumber, Video, VideoId};
    use crate::domain::video_category::{VideoCategories, VideoCategoriesChanged};
    use crate::domain::video_metadata::{VideoMetadata, render_episode_nfo};
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
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        PlaylistRepository, SqlitePlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::{
        PlaylistVideoRepository, SqlitePlaylistVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_categories_repository::{
        SqliteVideoCategoriesRepository, VideoCategoriesRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::{
        SqliteVideoMetadataRepository, VideoMetadataRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::{
        FakeYoutubeMetadataRepository, YoutubeMetadata,
    };
    use crate::infrastructure::shared::category_lock::CategoryLock;
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::{Database, TestDatabase};
    use crate::infrastructure::shared::system_clock::FixedClock;
    use crate::infrastructure::shared::video_lock::VideoLock;
    use chrono::{DateTime, Duration, Utc};
    use std::sync::Arc;

    const STRANDED_WARNING: &str = "stranded video found, scheduling its download";
    const DOWNLOADED_FILENAME: &str = "Season 2026/S2026E031501 - My Video.mp4";

    #[test]
    fn it_should_schedule_the_download_and_thumbnail_of_a_new_channel_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = my_video();
        video_repository.save(&video).unwrap();
        channel_repository
            .insert(&Channel {
                quality: Quality::Low,
                ..my_channel()
            })
            .unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(
                    1,
                    &Task::FetchThumbnail {
                        video_id: video.id.as_str().to_string(),
                        output_dir: "/videos/creators/somechannel".to_string(),
                    },
                ),
                pending_task(
                    2,
                    &Task::DownloadVideo {
                        video_id: video.id.as_str().to_string(),
                        quality: "low".to_string(),
                        output_dir: "/videos/creators/somechannel".to_string(),
                    },
                ),
            ]
        );
    }

    #[test]
    fn it_should_schedule_the_download_of_a_new_playlist_video_in_its_playlist_folder() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = my_video();
        video_repository.save(&video).unwrap();
        playlist_repository.insert(&my_playlist()).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                my_playlist_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            playlist_video_repository,
            Arc::new(SqliteChannelRepository::new(db.database())),
            playlist_repository,
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(
                    1,
                    &Task::FetchThumbnail {
                        video_id: video.id.as_str().to_string(),
                        output_dir: "/videos/music/chill".to_string(),
                    },
                ),
                pending_task(
                    2,
                    &Task::DownloadVideo {
                        video_id: video.id.as_str().to_string(),
                        quality: "mid".to_string(),
                        output_dir: "/videos/music/chill".to_string(),
                    },
                ),
            ]
        );
    }

    #[test]
    fn it_should_schedule_the_subtitles_of_a_downloaded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = downloaded_video();
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(vec![
                DOWNLOADED_FILENAME.to_string(),
            ])),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchSubtitles {
                    video_id: video.id.as_str().to_string(),
                    output_dir: "/videos/creators/somechannel".to_string(),
                },
            )]
        );
    }

    #[test]
    fn it_should_schedule_nothing_for_a_fully_synced_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = downloaded_video().with_subtitles(
            "Season 2026/S2026E031501 - My Video.en.srt",
            fixed_timestamp(),
        );
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        video_repository.update_subtitles(&video).unwrap();
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(vec![
                DOWNLOADED_FILENAME.to_string(),
            ])),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_schedule_a_second_download_if_one_is_queued() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = my_video();
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        task_repository
            .schedule(&download_video(&video), fixed_timestamp())
            .unwrap();
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(1, &download_video(&video)),
                pending_task(2, &fetch_thumbnail(&video)),
            ]
        );
    }

    #[test]
    fn it_should_not_schedule_a_thumbnail_for_a_video_being_downloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = my_video().start_download(fixed_timestamp());
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        task_repository
            .schedule(&download_video(&video), fixed_timestamp())
            .unwrap();
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(1, &download_video(&video))]
        );
    }

    #[test]
    fn it_should_schedule_nothing_for_a_settled_errored_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = my_video().mark_errored(fixed_timestamp());
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_categorize_a_new_video_of_a_categorized_channel() {
        let db = TestDatabase::new();
        let video_repository = SqliteVideoRepository::new(db.database());
        let video_categories_repository =
            Arc::new(SqliteVideoCategoriesRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video = my_video();
        save_channel_video(
            &video_repository,
            &SqliteChannelRepository::new(db.database()),
            &SqliteChannelVideoRepository::new(db.database()),
            &video,
        );
        let chess = category_with_members("Chess", &[CategoryMember::Channel(my_channel_id())]);
        SqliteCategoryRepository::new(db.database())
            .insert(&chess)
            .unwrap();
        let task = SyncVideoTask::new(VideoReconciler::new(
            Arc::new(video_repository),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                Arc::new(SqliteVideoRepository::new(db.database())),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            VideoCategoryAssigner::new(
                video_categories_repository.clone(),
                Arc::new(SqliteCategoryRepository::new(db.database())),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(CategoryLock::default()),
            ),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_categories_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![VideoCategories::new(
                video.id.clone(),
                vec![chess.id.clone()]
            )]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoCategoriesChanged(VideoCategoriesChanged {
                    video_ids: vec![video.id.as_str().to_string()],
                })
            )]
        );
    }

    #[test]
    fn it_should_leave_a_new_video_of_an_uncategorized_playlist_uncategorized() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_categories_repository =
            Arc::new(SqliteVideoCategoriesRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video = my_video();
        video_repository.save(&video).unwrap();
        SqlitePlaylistRepository::new(db.database())
            .insert(&my_playlist())
            .unwrap();
        SqlitePlaylistVideoRepository::new(db.database())
            .save(&PlaylistVideo::create(
                my_playlist_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        SqliteCategoryRepository::new(db.database())
            .insert(&category_with_members(
                "Chess",
                &[CategoryMember::Channel(my_channel_id())],
            ))
            .unwrap();
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            VideoCategoryAssigner::new(
                video_categories_repository.clone(),
                Arc::new(SqliteCategoryRepository::new(db.database())),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(CategoryLock::default()),
            ),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_categories_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![VideoCategories::none(video.id.clone())]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_if_video_is_gone() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let task = SyncVideoTask::new(VideoReconciler::new(
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                Arc::new(SqliteVideoRepository::new(db.database())),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&my_video()));

        assert_eq!(result, Ok(()));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_if_video_has_no_owner() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = my_video();
        video_repository.save(&video).unwrap();
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_reset_a_video_whose_file_is_missing() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = downloaded_video();
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(Vec::new())),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().reset_for_redownload(fixed_timestamp())]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(1, &fetch_thumbnail(&video)),
                pending_task(2, &download_video(&video)),
            ]
        );
    }

    #[test]
    fn it_should_reset_a_video_whose_file_is_not_mp4() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            filename: Some("Season 2026/S2026E031501 - My Video.mkv".to_string()),
            ..downloaded_video()
        };
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(vec![
                "Season 2026/S2026E031501 - My Video.mkv".to_string(),
            ])),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().reset_for_redownload(fixed_timestamp())]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(1, &fetch_thumbnail(&video)),
                pending_task(2, &download_video(&video)),
            ]
        );
    }

    #[test]
    fn it_should_recover_an_errored_video_after_its_cooldown() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = my_video()
            .start_download(a_day_ago())
            .mark_errored(a_day_ago());
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(Vec::new())),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().reset_for_redownload(fixed_timestamp())]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(1, &fetch_thumbnail(&video)),
                pending_task(2, &download_video(&video)),
            ]
        );
    }

    #[test]
    fn it_should_never_recover_an_excluded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = my_video()
            .start_download(a_day_ago() - Duration::days(30))
            .mark_excluded(a_day_ago() - Duration::days(30));
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(Vec::new())),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_generate_missing_metadata_of_a_playlist_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let videos_root = tempfile::tempdir().unwrap();
        let playlist_dir = videos_root.path().join("music/chill");
        std::fs::create_dir_all(playlist_dir.join("Season 2026")).unwrap();
        let video = downloaded_video();
        save_playlist_video(
            &video_repository,
            &playlist_repository,
            &playlist_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            playlist_video_repository,
            Arc::new(SqliteChannelRepository::new(db.database())),
            playlist_repository,
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(vec![
                DOWNLOADED_FILENAME.to_string(),
            ])),
            video_metadata_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository {
                    metadata: Some(youtube_metadata()),
                }),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                video_metadata_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            videos_root.path().to_string_lossy(),
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(
                VideoMetadata::new(
                    "My Video",
                    "A description",
                    "My Channel",
                    "My Channel",
                    fixed_timestamp(),
                    None,
                    Vec::new(),
                    "yt1",
                    None,
                    "0003 My Video",
                    fixed_timestamp(),
                )
                .with_thumb(Some("S2026E031501 - My Video.jpg".to_string()))
            )
        );
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchSubtitles {
                    video_id: video.id.as_str().to_string(),
                    output_dir: playlist_dir.to_string_lossy().to_string(),
                },
            )]
        );
    }

    #[test]
    fn it_should_generate_missing_metadata_of_a_channel_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let videos_root = tempfile::tempdir().unwrap();
        let channel_dir = videos_root.path().join("creators/somechannel");
        std::fs::create_dir_all(channel_dir.join("Season 2026")).unwrap();
        let video = downloaded_video();
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(vec![
                DOWNLOADED_FILENAME.to_string(),
            ])),
            video_metadata_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository {
                    metadata: Some(youtube_metadata()),
                }),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                video_metadata_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            videos_root.path().to_string_lossy(),
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(
                VideoMetadata::new(
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
                )
                .with_thumb(Some("S2026E031501 - My Video.jpg".to_string()))
            )
        );
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchSubtitles {
                    video_id: video.id.as_str().to_string(),
                    output_dir: channel_dir.to_string_lossy().to_string(),
                },
            )]
        );
    }

    #[test]
    fn it_should_keep_existing_metadata() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let videos_root = tempfile::tempdir().unwrap();
        let channel_dir = videos_root.path().join("creators/somechannel");
        std::fs::create_dir_all(channel_dir.join("Season 2026")).unwrap();
        let video = downloaded_video();
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let existing_metadata = VideoMetadata::new(
            "Old Title",
            "An old description",
            "My Channel",
            "My Channel",
            fixed_timestamp(),
            None,
            Vec::new(),
            "yt1",
            None,
            "20231114 Old Title",
            fixed_timestamp(),
        );
        video_metadata_repository
            .save(
                &video.id,
                &existing_metadata,
                &video.episode_number.unwrap(),
                &channel_dir.join("Season 2026/S2026E031501 - My Video.nfo"),
            )
            .unwrap();
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(vec![
                DOWNLOADED_FILENAME.to_string(),
            ])),
            video_metadata_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository {
                    metadata: Some(youtube_metadata()),
                }),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                video_metadata_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            videos_root.path().to_string_lossy(),
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(existing_metadata)
        );
    }

    #[test]
    fn it_should_relocate_a_legacy_video_when_its_metadata_is_recovered() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let relocated_files = Arc::new(FakeVideoFileRepository::with_file_exists(true));
        let videos_root = tempfile::tempdir().unwrap();
        let channel_dir = videos_root.path().join("creators/somechannel");
        std::fs::create_dir_all(channel_dir.join("My Video")).unwrap();
        let video = my_video()
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "My Video/My Video.mp4",
                Some("My Video/My Video.jpg".to_string()),
                None,
                fixed_timestamp(),
            );
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            video_metadata_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository {
                    metadata: Some(youtube_metadata()),
                }),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                relocated_files.clone(),
                video_metadata_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            videos_root.path().to_string_lossy(),
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().relocate_to_episode(
                recovered_episode(),
                "Season 2023/S2023E111401 - My Video.mp4",
                Some("Season 2023/S2023E111401 - My Video.jpg".to_string()),
                fixed_timestamp(),
            )]
        );
        assert_eq!(
            *relocated_files.moved.lock().unwrap(),
            vec![
                (
                    channel_dir.join("My Video/My Video.mp4"),
                    channel_dir.join("Season 2023/S2023E111401 - My Video.mp4"),
                ),
                (
                    channel_dir.join("My Video/My Video.jpg"),
                    channel_dir.join("Season 2023/S2023E111401 - My Video.jpg"),
                ),
            ]
        );
        assert_eq!(
            *relocated_files.removed_if_empty.lock().unwrap(),
            vec![channel_dir.join("My Video")]
        );
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(recovered_metadata())
        );
        assert_eq!(
            std::fs::read_to_string(channel_dir.join("Season 2023/S2023E111401 - My Video.nfo"))
                .unwrap(),
            render_episode_nfo(&recovered_metadata(), &recovered_episode())
        );
    }

    #[test]
    fn it_should_leave_a_legacy_video_in_place_when_its_metadata_is_still_unavailable() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let relocated_files = Arc::new(FakeVideoFileRepository::with_file_exists(true));
        let video = my_video()
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "My Video/My Video.mp4",
                Some("My Video/My Video.jpg".to_string()),
                None,
                fixed_timestamp(),
            );
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            video_metadata_repository.clone(),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                relocated_files.clone(),
                video_metadata_repository.clone(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let result = run(&task, &payload_for(&video));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(*relocated_files.moved.lock().unwrap(), vec![]);
        assert_eq!(video_metadata_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_warn_about_a_stranded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = my_video();
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(Vec::new())),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let mut result = Err(String::new());
        let logs = captured_log_messages(|| result = run(&task, &payload_for(&video)));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(1, &fetch_thumbnail(&video)),
                pending_task(2, &download_video(&video)),
            ]
        );
        assert!(logs.contains(&STRANDED_WARNING.to_string()));
    }

    #[test]
    fn it_should_keep_a_stranded_errored_retrying_video_errored_retrying() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = my_video()
            .start_download(fixed_timestamp())
            .mark_errored_retrying(fixed_timestamp());
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(Vec::new())),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let mut result = Err(String::new());
        let logs = captured_log_messages(|| result = run(&task, &payload_for(&video)));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(1, &fetch_thumbnail(&video)),
                pending_task(2, &download_video(&video)),
            ]
        );
        assert!(logs.contains(&STRANDED_WARNING.to_string()));
    }

    #[test]
    fn it_should_not_warn_about_a_video_whose_download_is_running() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = my_video().start_download(fixed_timestamp());
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        task_repository
            .schedule(&download_video(&video), fixed_timestamp())
            .unwrap();
        task_repository.claim(1, fixed_timestamp()).unwrap();
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            channel_video_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::with_listing(Vec::new())),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoLock::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
            video_category_assigner(db.database()),
        ));

        let mut result = Err(String::new());
        let logs = captured_log_messages(|| result = run(&task, &payload_for(&video)));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![ScheduledTask {
                status: TaskStatus::Running,
                ..pending_task(1, &download_video(&video))
            }]
        );
        assert!(!logs.contains(&STRANDED_WARNING.to_string()));
    }

    fn my_video() -> Video {
        Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp())
    }

    /// Downloaded and numbered in its season folder, with its thumbnail
    /// recorded and its subtitles still Missing.
    fn downloaded_video() -> Video {
        my_video()
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                DOWNLOADED_FILENAME,
                Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                None,
                fixed_timestamp(),
            )
            .with_episode_number(
                EpisodeNumber::from_columns(2026, 31501).unwrap(),
                fixed_timestamp(),
            )
    }

    /// Stores `video` as the only video of `my_channel()`.
    fn save_channel_video(
        video_repository: &SqliteVideoRepository,
        channel_repository: &SqliteChannelRepository,
        channel_video_repository: &SqliteChannelVideoRepository,
        video: &Video,
    ) {
        video_repository.save(video).unwrap();
        video_repository.update(video).unwrap();
        channel_repository.insert(&my_channel()).unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    /// Stores `video` as the only video of `my_playlist()`, at position 3.
    fn save_playlist_video(
        video_repository: &SqliteVideoRepository,
        playlist_repository: &SqlitePlaylistRepository,
        playlist_video_repository: &SqlitePlaylistVideoRepository,
        video: &Video,
    ) {
        video_repository.save(video).unwrap();
        video_repository.update(video).unwrap();
        playlist_repository.insert(&my_playlist()).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                my_playlist_id(),
                video.id.clone(),
                3,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    fn my_channel_id() -> ChannelHandle {
        ChannelHandle::new("@somechannel").unwrap()
    }

    fn my_channel() -> Channel {
        Channel::create(
            my_channel_id(),
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

    fn my_playlist_id() -> PlaylistId {
        PlaylistId::new("PL1").unwrap()
    }

    fn my_playlist() -> Playlist {
        Playlist::create(
            my_playlist_id(),
            PlaylistName::new("Chill").unwrap(),
            PlaylistPath::new("music/chill").unwrap(),
            Quality::Mid,
            PlaylistKind::YoutubeLinked,
            false,
            ReconcileInterval::default_interval(),
            fixed_timestamp(),
        )
    }

    fn category_with_members(name: &str, members: &[CategoryMember]) -> Category {
        Category::create(
            CategoryName::new(name).unwrap(),
            CategoryColor::new(CategoryColor::PALETTE[0]).unwrap(),
            fixed_timestamp(),
        )
        .with_members(members.to_vec())
    }

    fn youtube_metadata() -> YoutubeMetadata {
        YoutubeMetadata {
            title: "My Video".to_string(),
            description: "A description".to_string(),
            channel_title: "My Channel".to_string(),
            published_at: fixed_timestamp(),
            tags: Vec::new(),
            category_id: None,
        }
    }

    /// The episode a legacy `my_video()` of `my_channel()` is relocated to.
    fn recovered_episode() -> EpisodeNumber {
        EpisodeNumber::from_columns(2023, 111401).unwrap()
    }

    /// The metadata a sync recovers for a legacy `my_video()` once relocated.
    fn recovered_metadata() -> VideoMetadata {
        VideoMetadata::new(
            "My Video",
            "A description",
            "My Channel",
            "My Channel",
            fixed_timestamp(),
            None,
            Vec::new(),
            "yt1",
            Some("S2023E111401 - My Video.jpg".to_string()),
            "20231114 My Video",
            fixed_timestamp(),
        )
    }

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

    fn payload_for(video: &Video) -> String {
        Task::SyncVideo {
            video_id: video.id.as_str().to_string(),
        }
        .payload()
        .to_string()
    }

    /// The download `my_channel()` wants for `video`.
    fn download_video(video: &Video) -> Task {
        Task::DownloadVideo {
            video_id: video.id.as_str().to_string(),
            quality: "high".to_string(),
            output_dir: "/videos/creators/somechannel".to_string(),
        }
    }

    fn fetch_thumbnail(video: &Video) -> Task {
        Task::FetchThumbnail {
            video_id: video.id.as_str().to_string(),
            output_dir: "/videos/creators/somechannel".to_string(),
        }
    }

    fn pending_task(id: i64, task: &Task) -> ScheduledTask {
        ScheduledTask {
            id,
            task_type: task.task_type().to_string(),
            payload: task.payload().to_string(),
            status: TaskStatus::Pending,
            retries: 0,
            run_at: fixed_timestamp(),
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn a_day_ago() -> DateTime<Utc> {
        fixed_timestamp() - Duration::hours(24)
    }

    fn run(task: &SyncVideoTask, payload: &str) -> Result<(), String> {
        task.handle(payload, false).map_err(|e| e.to_string())
    }

    fn video_category_assigner(database: Database) -> VideoCategoryAssigner {
        VideoCategoryAssigner::new(
            Arc::new(SqliteVideoCategoriesRepository::new(database.clone())),
            Arc::new(SqliteCategoryRepository::new(database.clone())),
            Arc::new(SqliteChannelVideoRepository::new(database.clone())),
            Arc::new(SqlitePlaylistVideoRepository::new(database.clone())),
            Arc::new(SqliteEventPublisher::new(
                database,
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(CategoryLock::default()),
        )
    }
}
