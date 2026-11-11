use crate::domain::services::{VideoReconciler, VideoReconcilerApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};

/// Syncs one stored video: brings it to its desired state and schedules
/// whichever of its steps it still needs (`VideoReconciler`). Scheduled by
/// the `SyncVideoOn*` subscribers when a video is added to a playlist or
/// channel and each time one is downloaded.
pub struct SyncVideoTask {
    video_reconciler: VideoReconciler,
}

impl SyncVideoTask {
    pub fn new(video_reconciler: VideoReconciler) -> Self {
        Self { video_reconciler }
    }
}

impl TaskHandler for SyncVideoTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::SyncVideo { video_id } = task else {
            return Err(unexpected_task(task));
        };
        let video_id = VideoId::new(video_id.clone())?;
        self.video_reconciler
            .reconcile(&video_id)
            .map(|()| TaskOutcome::Done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tasks::log_capture::captured_log_messages;
    use crate::domain::category::{
        Category, CategoryColor, CategoryMember, CategoryName, CategoryPrompt,
    };
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::llm_server::OllamaUrl;
    use crate::domain::playlist::{Playlist, PlaylistId, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::preferences::{CategorizationSettings, Preferences, SummarySettings};
    use crate::domain::services::MetadataGenerator;
    use crate::domain::services::VideoCategorizer;
    use crate::domain::services::VideoCategoryAssigner;
    use crate::domain::services::VideoOwnerFinder;
    use crate::domain::services::VideoRelocator;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::domain::video::Chapters;
    use crate::domain::video::Download;
    use crate::domain::video::DownloadedQuality;
    use crate::domain::video::StepStatus;
    use crate::domain::video::Subtitles;
    use crate::domain::video::Thumbnail;
    use crate::domain::video::{EpisodeNumber, StepState, Video, VideoChapter, YoutubeVideoId};
    use crate::domain::video_category::{
        Verdict, VerdictAuthor, VideoCategories, VideoCategoriesChanged, VideoCategoryVerdict,
    };
    use crate::domain::video_metadata::{VideoMetadata, render_episode_nfo};
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::ollama_llm_server_repository::FakeLlmServerRepository;
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
    use crate::infrastructure::repositories::sqlite_preferences_repository::{
        PreferencesRepository, SqlitePreferencesRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_categories_repository::{
        SqliteVideoCategoriesRepository, VideoCategoriesRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::{
        SqliteVideoCategoryVerdictRepository, VideoCategoryVerdictRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::{
        SqliteVideoMetadataRepository, VideoMetadataRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_summary_repository::SqliteVideoSummaryRepository;
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
    use chrono::{DateTime, Duration, Utc};
    use std::sync::Arc;

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
        video_repository.insert(&video).unwrap();
        channel_repository.insert(&my_channel()).unwrap();
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(
                    1,
                    &Task::FetchThumbnail {
                        video_id: video.id.as_str().to_string(),
                    },
                ),
                pending_task(
                    2,
                    &Task::DownloadVideo {
                        video_id: video.id.as_str().to_string(),
                    },
                ),
            ]
        );
    }

    #[test]
    fn it_should_schedule_the_download_and_thumbnail_of_a_new_playlist_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = my_video();
        video_repository.insert(&video).unwrap();
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
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                playlist_video_repository,
                Arc::new(SqliteChannelRepository::new(db.database())),
                playlist_repository,
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(
                    1,
                    &Task::FetchThumbnail {
                        video_id: video.id.as_str().to_string(),
                    },
                ),
                pending_task(
                    2,
                    &Task::DownloadVideo {
                        video_id: video.id.as_str().to_string(),
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchSubtitles {
                    video_id: video.id.as_str().to_string(),
                },
            )]
        );
    }

    #[test]
    fn it_should_schedule_subtitles_checked_more_than_a_day_ago() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            subtitles: Subtitles::checked(Some(fixed_timestamp() - Duration::hours(25)), 0),
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchSubtitles {
                    video_id: video.id.as_str().to_string(),
                },
            )]
        );
    }

    #[test]
    fn it_should_not_schedule_subtitles_checked_less_than_a_day_ago() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            subtitles: Subtitles::checked(Some(fixed_timestamp() - Duration::hours(3)), 0),
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_schedule_the_chapters_fetch_of_a_downloaded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            chapters: Chapters::pending(),
            ..downloaded_video().mark_subtitles_unavailable(fixed_timestamp())
        };
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchChapters {
                    video_id: video.id.as_str().to_string(),
                },
            )]
        );
    }

    #[test]
    fn it_should_schedule_chapters_of_a_young_video_checked_a_day_ago() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            chapters: Chapters {
                found: Some(vec![]),
                ..Chapters::checked(Some(fixed_timestamp() - Duration::hours(25)), 0)
            },
            ..downloaded_video().mark_subtitles_unavailable(fixed_timestamp())
        };
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchChapters {
                    video_id: video.id.as_str().to_string(),
                },
            )]
        );
    }

    #[test]
    fn it_should_schedule_a_quality_check_when_due() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            downloaded_quality: DownloadedQuality::checked(
                Some(fixed_timestamp() - Duration::hours(25)),
                0,
            ),
            ..downloaded_video().mark_subtitles_unavailable(fixed_timestamp())
        };
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::UpgradeQuality {
                    video_id: video.id.as_str().to_string(),
                },
            )]
        );
    }

    #[test]
    fn it_should_not_schedule_a_quality_check_right_after_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            downloaded_quality: DownloadedQuality::checked(
                Some(fixed_timestamp() - Duration::minutes(10)),
                0,
            ),
            ..downloaded_video().mark_subtitles_unavailable(fixed_timestamp())
        };
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_schedule_chapters_checked_less_than_a_day_ago() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            chapters: Chapters {
                found: Some(vec![]),
                ..Chapters::checked(Some(fixed_timestamp() - Duration::hours(3)), 0)
            },
            ..downloaded_video().mark_subtitles_unavailable(fixed_timestamp())
        };
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_schedule_settled_chapters() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            chapters: Chapters {
                found: Some(vec![]),
                ..Chapters::done_at(Some(fixed_timestamp() - Duration::days(30)))
            },
            ..downloaded_video().mark_subtitles_unavailable(fixed_timestamp())
        };
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_schedule_a_chapters_fetch_if_already_checked() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            chapters: Chapters {
                found: Some(vec![VideoChapter {
                    start_seconds: 0,
                    end_seconds: 60,
                    title: "Intro".to_string(),
                }]),
                ..downloaded_video().chapters
            },
            ..downloaded_video().mark_subtitles_unavailable(fixed_timestamp())
        };
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
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
        let video = downloaded_video().mark_subtitles_fetched(
            "Season 2026/S2026E031501 - My Video.en.srt",
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(1, &download_video(&video))]
        );
    }

    #[test]
    fn it_should_schedule_the_download_of_a_retrying_video_once_due() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            thumbnail: Thumbnail {
                filename: Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                ..Thumbnail::pending()
            },
            ..my_video()
                .mark_errored("HTTP Error 503", fixed_timestamp() - Duration::days(2))
                .mark_errored("HTTP Error 503", fixed_timestamp() - Duration::days(2))
        };
        seed_channel_video(&db, &video_repository, &video);
        let task = sync_task(&db, video_repository.clone(), task_repository.clone());

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..video.clone()
            }]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(1, &download_video(&video))]
        );
    }

    #[test]
    fn it_should_not_schedule_steps_that_are_not_due() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let three_minutes_ago = fixed_timestamp() - Duration::minutes(3);
        let video = my_video()
            .mark_errored("HTTP Error 503", three_minutes_ago)
            .record_thumbnail_failure(three_minutes_ago)
            .record_summary_failure(three_minutes_ago);
        seed_channel_video(&db, &video_repository, &video);
        let task = sync_task(&db, video_repository.clone(), task_repository.clone());

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..video.clone()
            }]
        );
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_schedule_failed_steps() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let long_ago = fixed_timestamp() - Duration::days(30);
        let video = Video {
            subtitles: Subtitles::checked(Some(long_ago), 11),
            chapters: Chapters {
                found: Some(vec![]),
                ..Chapters::checked(Some(long_ago), 11)
            },
            downloaded_quality: DownloadedQuality::checked(Some(long_ago), 11),
            thumbnail: Thumbnail {
                filename: None,
                state: (0..11).fold(StepState::pending(), |state, _| state.fail(long_ago)),
            },
            ..downloaded_video()
        };
        seed_channel_video(&db, &video_repository, &video);
        let task = sync_task(&db, video_repository.clone(), task_repository.clone());

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_schedule_retrying_steps_once_their_next_attempt_is_due() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let ten_minutes_ago = fixed_timestamp() - Duration::minutes(10);
        let video = Video {
            subtitles: Subtitles::checked(Some(ten_minutes_ago), 1),
            thumbnail: Thumbnail {
                filename: None,
                state: StepState::pending().fail(ten_minutes_ago),
            },
            ..downloaded_video()
        };
        seed_channel_video(&db, &video_repository, &video);
        let task = sync_task(&db, video_repository.clone(), task_repository.clone());

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(1, &fetch_thumbnail(&video)),
                pending_task(2, &fetch_subtitles(&video)),
            ]
        );
    }

    #[test]
    fn it_should_schedule_retrying_chapters_once_their_next_attempt_is_due() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            chapters: Chapters::checked(Some(fixed_timestamp() - Duration::minutes(10)), 1),
            ..downloaded_video().mark_subtitles_unavailable(fixed_timestamp())
        };
        seed_channel_video(&db, &video_repository, &video);
        let task = sync_task(&db, video_repository.clone(), task_repository.clone());

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchChapters {
                    video_id: video.id.as_str().to_string(),
                },
            )]
        );
    }

    #[test]
    fn it_should_schedule_a_retrying_quality_check_once_its_next_attempt_is_due() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            downloaded_quality: DownloadedQuality::checked(
                Some(fixed_timestamp() - Duration::minutes(10)),
                1,
            ),
            ..downloaded_video().mark_subtitles_unavailable(fixed_timestamp())
        };
        seed_channel_video(&db, &video_repository, &video);
        let task = sync_task(&db, video_repository.clone(), task_repository.clone());

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::UpgradeQuality {
                    video_id: video.id.as_str().to_string(),
                },
            )]
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
        let video = my_video().mark_download_failed("HTTP Error 503", fixed_timestamp());
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..video.clone()
            }]
        );
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_categorize_a_new_video_of_a_categorized_channel() {
        let db = TestDatabase::new();
        let video_repository = SqliteVideoRepository::new(db.database());
        let video_categories_repository =
            Arc::new(SqliteVideoCategoriesRepository::new(db.database()));
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
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
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            VideoCategoryAssigner::new(
                Arc::new(SqliteVideoRepository::new(db.database())),
                Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
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
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
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
    fn it_should_give_a_video_its_source_categories_plus_its_yes_verdicts() {
        let db = TestDatabase::new();
        let video_repository = SqliteVideoRepository::new(db.database());
        let category_repository = SqliteCategoryRepository::new(db.database());
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video_categories_repository =
            Arc::new(SqliteVideoCategoriesRepository::new(db.database()));
        let video = my_video();
        save_channel_video(
            &video_repository,
            &SqliteChannelRepository::new(db.database()),
            &SqliteChannelVideoRepository::new(db.database()),
            &video,
        );
        let chess = category_with_members("Chess", &[CategoryMember::Channel(my_channel_id())]);
        let (sicilian, tech) = (
            category_with_members("Sicilian", &[]),
            category_with_members("Tech", &[]),
        );
        [&chess, &sicilian, &tech]
            .iter()
            .for_each(|category| category_repository.insert(category).unwrap());
        verdict_repository
            .save(&ai_verdict(&video, &sicilian, Verdict::Yes))
            .unwrap();
        verdict_repository
            .save(&ai_verdict(&video, &tech, Verdict::No))
            .unwrap();
        let task = SyncVideoTask::new(VideoReconciler::new(
            Arc::new(video_repository),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            VideoCategoryAssigner::new(
                Arc::new(SqliteVideoRepository::new(db.database())),
                Arc::new(verdict_repository),
                video_categories_repository.clone(),
                Arc::new(category_repository),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(CategoryLock::default()),
            ),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_categories_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![VideoCategories::new(
                video.id.clone(),
                vec![chess.id.clone(), sicilian.id.clone()]
            )]
        );
    }

    #[test]
    fn it_should_give_a_video_of_an_uncategorized_source_its_yes_verdicts() {
        let db = TestDatabase::new();
        let video_repository = SqliteVideoRepository::new(db.database());
        let category_repository = SqliteCategoryRepository::new(db.database());
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let video_categories_repository =
            Arc::new(SqliteVideoCategoriesRepository::new(db.database()));
        let video = my_video();
        save_channel_video(
            &video_repository,
            &SqliteChannelRepository::new(db.database()),
            &SqliteChannelVideoRepository::new(db.database()),
            &video,
        );
        let (chess, funny) = (
            category_with_members("Chess", &[]),
            category_with_members("Funny", &[]),
        );
        [&chess, &funny]
            .iter()
            .for_each(|category| category_repository.insert(category).unwrap());
        verdict_repository
            .save(&ai_verdict(&video, &chess, Verdict::Yes))
            .unwrap();
        verdict_repository
            .save(&ai_verdict(&video, &funny, Verdict::No))
            .unwrap();
        let task = SyncVideoTask::new(VideoReconciler::new(
            Arc::new(video_repository),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            VideoCategoryAssigner::new(
                Arc::new(SqliteVideoRepository::new(db.database())),
                Arc::new(verdict_repository),
                video_categories_repository.clone(),
                Arc::new(category_repository),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(CategoryLock::default()),
            ),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_categories_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![VideoCategories::new(
                video.id.clone(),
                vec![chess.id.clone()]
            )]
        );
    }

    #[test]
    fn it_should_leave_a_new_video_of_an_uncategorized_playlist_uncategorized() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_categories_repository =
            Arc::new(SqliteVideoCategoriesRepository::new(db.database()));
        let event_repository =
            SqliteEventRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
        let video = my_video();
        video_repository.insert(&video).unwrap();
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
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            VideoCategoryAssigner::new(
                Arc::new(SqliteVideoRepository::new(db.database())),
                Arc::new(SqliteVideoCategoryVerdictRepository::new(db.database())),
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
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
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
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&my_video()));

        assert_eq!(result, Ok(TaskOutcome::Done));
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
        video_repository.insert(&video).unwrap();
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                ..video.clone().reset_for_redownload(fixed_timestamp())
            }]
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
    fn it_should_reset_every_check_on_redownload() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = Video {
            subtitles: Subtitles::checked(Some(fixed_timestamp() - Duration::hours(1)), 2),
            chapters: Chapters {
                found: Some(vec![]),
                ..Chapters::checked(Some(fixed_timestamp() - Duration::hours(1)), 2)
            },
            downloaded_quality: DownloadedQuality {
                height: Some(720),
                ..DownloadedQuality::checked(Some(fixed_timestamp() - Duration::hours(1)), 2)
            },
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                synced_at: None,
                updated_at: fixed_timestamp(),
                subtitles: Subtitles::pending(),
                chapters: Chapters {
                    found: video.chapters.found.clone(),
                    ..Chapters::pending()
                },
                downloaded_quality: DownloadedQuality::pending(),
                thumbnail: Thumbnail::pending(),
                download: Download {
                    started: false,
                    state: StepState {
                        status: StepStatus::Pending,
                        ..video.download.clone().state
                    },
                    file: None,
                    ..video.download.clone()
                },
                ..video.clone()
            }]
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
        let video = downloaded_video()
            .map_file(|file| file.with_filename("Season 2026/S2026E031501 - My Video.mkv"));
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                ..video.clone().reset_for_redownload(fixed_timestamp())
            }]
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
            .mark_unavailable(a_day_ago() - Duration::days(30));
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..video.clone()
            }]
        );
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
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                playlist_video_repository,
                Arc::new(SqliteChannelRepository::new(db.database())),
                playlist_repository,
                videos_root.path().to_string_lossy(),
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
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
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..video.clone()
            }]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchSubtitles {
                    video_id: video.id.as_str().to_string(),
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                videos_root.path().to_string_lossy(),
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
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
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..video.clone()
            }]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::FetchSubtitles {
                    video_id: video.id.as_str().to_string(),
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                videos_root.path().to_string_lossy(),
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                videos_root.path().to_string_lossy(),
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                ..video.clone().relocate_to_episode(
                    recovered_episode(),
                    "Season 2023/S2023E111401 - My Video.mp4",
                    Some("Season 2023/S2023E111401 - My Video.jpg".to_string()),
                    fixed_timestamp(),
                )
            }]
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..video.clone()
            }]
        );
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
        let video = my_video().start_download(fixed_timestamp());
        save_channel_video(
            &video_repository,
            &channel_repository,
            &channel_video_repository,
            &video,
        );
        let task = SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let mut result = Err(String::new());
        let logs = captured_log_messages(|| result = run(&task, &task_for(&video)));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..video.clone()
            }]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(1, &download_video(&video))]
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let mut result = Err(String::new());
        let logs = captured_log_messages(|| result = run(&task, &task_for(&video)));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..video.clone()
            }]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![ScheduledTask {
                status: TaskStatus::Running,
                ..pending_task(1, &download_video(&video))
            }]
        );
        assert!(!logs.contains(&STRANDED_WARNING.to_string()));
    }

    #[test]
    fn it_should_not_warn_about_a_new_video() {
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
            Arc::new(VideoOwnerFinder::new(
                channel_video_repository,
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                channel_repository,
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
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
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ));

        let mut result = Err(String::new());
        let logs = captured_log_messages(|| result = run(&task, &task_for(&video)));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(1, &fetch_thumbnail(&video)),
                pending_task(2, &download_video(&video)),
            ]
        );
        assert!(!logs.contains(&STRANDED_WARNING.to_string()));
    }

    const STRANDED_WARNING: &str = "stranded video found, scheduling its download";
    const DOWNLOADED_FILENAME: &str = "Season 2026/S2026E031501 - My Video.mp4";

    #[test]
    fn it_should_schedule_a_summary_for_a_subtitled_video_in_an_opted_in_source() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = subtitled_video();
        video_repository.insert(&video).unwrap();
        seed_channel_with_summaries(&db, &video, true);
        preferences_repository.save(&summaries_on()).unwrap();
        let task = sync_task_of_downloaded_video(
            &db,
            video_repository,
            task_repository.clone(),
            preferences_repository,
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(1, &summarize_video(&video))]
        );
    }

    #[test]
    fn it_should_not_schedule_a_summary_when_the_source_opted_out() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = subtitled_video();
        video_repository.insert(&video).unwrap();
        seed_channel_with_summaries(&db, &video, false);
        preferences_repository.save(&summaries_on()).unwrap();
        let task = sync_task_of_downloaded_video(
            &db,
            video_repository,
            task_repository.clone(),
            preferences_repository,
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_schedule_a_summary_while_ai_summaries_are_off() {
        let ai_off = Preferences {
            ai_enabled: false,
            ..summaries_on()
        };
        let summaries_off = Preferences {
            summaries: SummarySettings {
                enabled: false,
                ..summaries_on().summaries
            },
            ..summaries_on()
        };

        let scheduled = [ai_off, summaries_off].map(|preferences| {
            let db = TestDatabase::new();
            let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
            let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
            let task_repository = Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            ));
            let video = subtitled_video();
            video_repository.insert(&video).unwrap();
            seed_channel_with_summaries(&db, &video, true);
            preferences_repository.save(&preferences).unwrap();
            let task = sync_task_of_downloaded_video(
                &db,
                video_repository,
                task_repository.clone(),
                preferences_repository,
            );

            run(&task, &task_for(&video)).unwrap();

            task_repository.list_non_completed().unwrap()
        });

        assert_eq!(scheduled, [vec![], vec![]]);
    }

    #[test]
    fn it_should_not_schedule_a_summary_without_fetched_subtitles() {
        let missing = downloaded_video();
        let unavailable = Video {
            subtitles: Subtitles::unavailable_at(None),
            ..downloaded_video()
        };

        let scheduled = [missing.clone(), unavailable].map(|video| {
            let db = TestDatabase::new();
            let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
            let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
            let task_repository = Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            ));
            video_repository.insert(&video).unwrap();
            seed_channel_with_summaries(&db, &video, true);
            preferences_repository.save(&summaries_on()).unwrap();
            let task = sync_task_of_downloaded_video(
                &db,
                video_repository,
                task_repository.clone(),
                preferences_repository,
            );

            run(&task, &task_for(&video)).unwrap();

            task_repository.list_non_completed().unwrap()
        });

        assert_eq!(
            scheduled,
            [vec![pending_task(1, &fetch_subtitles(&missing))], vec![]]
        );
    }

    #[test]
    fn it_should_not_schedule_a_settled_summary() {
        let scheduled = [
            StepState::pending().succeed(),
            (0..11).fold(StepState::pending(), |state, _| {
                state.fail(fixed_timestamp())
            }),
        ]
        .map(|summary| {
            let db = TestDatabase::new();
            let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
            let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
            let task_repository = Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            ));
            let video = Video {
                summary,
                ..subtitled_video()
            };
            video_repository.insert(&video).unwrap();
            seed_channel_with_summaries(&db, &video, true);
            preferences_repository.save(&summaries_on()).unwrap();
            let task = sync_task_of_downloaded_video(
                &db,
                video_repository,
                task_repository.clone(),
                preferences_repository,
            );

            run(&task, &task_for(&video)).unwrap();

            task_repository.list_non_completed().unwrap()
        });

        assert_eq!(scheduled, [vec![], vec![]]);
    }

    #[test]
    fn it_should_schedule_a_categorization_for_a_summarized_video_with_pending_categories() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = summarized_video();
        video_repository.insert(&video).unwrap();
        seed_channel_with_categorization(&db, &video, true);
        SqliteCategoryRepository::new(db.database())
            .insert(&prompted_category("Sicilian", &[]))
            .unwrap();
        preferences_repository.save(&categorization_on()).unwrap();
        let task = sync_task_of_downloaded_video(
            &db,
            video_repository,
            task_repository.clone(),
            preferences_repository,
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(1, &categorize_video(&video))]
        );
    }

    #[test]
    fn it_should_not_schedule_a_categorization_without_a_generated_summary() {
        let videos = [
            subtitled_video(),
            subtitled_video().mark_summary_failed(fixed_timestamp()),
        ];

        let scheduled = videos.map(|video| {
            let db = TestDatabase::new();
            let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
            let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
            let task_repository = Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            ));
            video_repository.insert(&video).unwrap();
            seed_channel_with_categorization(&db, &video, true);
            SqliteCategoryRepository::new(db.database())
                .insert(&prompted_category("Sicilian", &[]))
                .unwrap();
            preferences_repository.save(&categorization_on()).unwrap();
            let task = sync_task_of_downloaded_video(
                &db,
                video_repository,
                task_repository.clone(),
                preferences_repository,
            );

            run(&task, &task_for(&video)).unwrap();

            task_repository.list_non_completed().unwrap()
        });

        assert_eq!(scheduled, [vec![], vec![]]);
    }

    #[test]
    fn it_should_not_schedule_a_categorization_when_the_source_has_categorization_off() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = summarized_video();
        video_repository.insert(&video).unwrap();
        seed_channel_with_categorization(&db, &video, false);
        SqliteCategoryRepository::new(db.database())
            .insert(&prompted_category("Sicilian", &[]))
            .unwrap();
        preferences_repository.save(&categorization_on()).unwrap();
        let task = sync_task_of_downloaded_video(
            &db,
            video_repository,
            task_repository.clone(),
            preferences_repository,
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_schedule_a_categorization_when_ai_categorization_is_off() {
        let ai_off = Preferences {
            ai_enabled: false,
            ..categorization_on()
        };
        let categories_off = Preferences {
            categories_enabled: false,
            ..categorization_on()
        };
        let categorization_off = Preferences {
            categorization: CategorizationSettings {
                enabled: false,
                ..categorization_on().categorization
            },
            ..categorization_on()
        };

        let scheduled = [ai_off, categories_off, categorization_off].map(|preferences| {
            let db = TestDatabase::new();
            let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
            let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
            let task_repository = Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            ));
            let video = summarized_video();
            video_repository.insert(&video).unwrap();
            seed_channel_with_categorization(&db, &video, true);
            SqliteCategoryRepository::new(db.database())
                .insert(&prompted_category("Sicilian", &[]))
                .unwrap();
            preferences_repository.save(&preferences).unwrap();
            let task = sync_task_of_downloaded_video(
                &db,
                video_repository,
                task_repository.clone(),
                preferences_repository,
            );

            run(&task, &task_for(&video)).unwrap();

            task_repository.list_non_completed().unwrap()
        });

        assert_eq!(scheduled, [vec![], vec![], vec![]]);
    }

    #[test]
    fn it_should_not_schedule_a_categorization_when_every_category_has_a_verdict() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let category_repository = SqliteCategoryRepository::new(db.database());
        let verdict_repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = summarized_video();
        video_repository.insert(&video).unwrap();
        seed_channel_with_categorization(&db, &video, true);
        [
            (prompted_category("Sicilian", &[]), Verdict::Yes),
            (prompted_category("Tech", &[]), Verdict::No),
            (prompted_category("Food", &[]), Verdict::Failed),
        ]
        .iter()
        .for_each(|(category, verdict)| {
            category_repository.insert(category).unwrap();
            verdict_repository
                .save(&ai_verdict(&video, category, *verdict))
                .unwrap();
        });
        preferences_repository.save(&categorization_on()).unwrap();
        let task = sync_task_of_downloaded_video(
            &db,
            video_repository,
            task_repository.clone(),
            preferences_repository,
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_count_a_category_given_by_the_source_or_without_a_prompt_as_pending() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let category_repository = SqliteCategoryRepository::new(db.database());
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = summarized_video();
        video_repository.insert(&video).unwrap();
        seed_channel_with_categorization(&db, &video, true);
        [
            prompted_category("Chess", &[CategoryMember::Channel(my_channel_id())]),
            category_with_members("Funny", &[]),
        ]
        .iter()
        .for_each(|category| category_repository.insert(category).unwrap());
        preferences_repository.save(&categorization_on()).unwrap();
        let task = sync_task_of_downloaded_video(
            &db,
            video_repository,
            task_repository.clone(),
            preferences_repository,
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_schedule_a_second_categorization_while_one_is_pending() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = summarized_video();
        video_repository.insert(&video).unwrap();
        seed_channel_with_categorization(&db, &video, true);
        SqliteCategoryRepository::new(db.database())
            .insert(&prompted_category("Sicilian", &[]))
            .unwrap();
        preferences_repository.save(&categorization_on()).unwrap();
        task_repository
            .schedule(&categorize_video(&video), fixed_timestamp())
            .unwrap();
        let task = sync_task_of_downloaded_video(
            &db,
            video_repository,
            task_repository.clone(),
            preferences_repository,
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(1, &categorize_video(&video))]
        );
    }

    #[test]
    fn it_should_not_schedule_a_categorization_that_failed_recently() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let preferences_repository = Arc::new(SqlitePreferencesRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video = summarized_video()
            .record_categorization_failure(fixed_timestamp() - Duration::minutes(3));
        video_repository.insert(&video).unwrap();
        seed_channel_with_categorization(&db, &video, true);
        SqliteCategoryRepository::new(db.database())
            .insert(&prompted_category("Sicilian", &[]))
            .unwrap();
        preferences_repository.save(&categorization_on()).unwrap();
        let task = sync_task_of_downloaded_video(
            &db,
            video_repository,
            task_repository.clone(),
            preferences_repository,
        );

        let result = run(&task, &task_for(&video));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    /// A sync task for a video whose file `DOWNLOADED_FILENAME` is on disk.
    fn sync_task_of_downloaded_video(
        db: &TestDatabase,
        video_repository: Arc<SqliteVideoRepository>,
        task_repository: Arc<SqliteTaskRepository>,
        preferences_repository: Arc<SqlitePreferencesRepository>,
    ) -> SyncVideoTask {
        SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            task_repository,
            Arc::new(FakeVideoFileRepository::with_listing(vec![
                DOWNLOADED_FILENAME.to_string(),
            ])),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository,
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            preferences_repository,
            video_categorizer(db.database()),
        ))
    }

    /// Stores `my_channel()`, with summaries on or off, as the owner of `video`.
    fn seed_channel_with_summaries(db: &TestDatabase, video: &Video, summaries_enabled: bool) {
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel().with_summaries_enabled(summaries_enabled))
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    fn seed_channel_with_categorization(
        db: &TestDatabase,
        video: &Video,
        categorization_enabled: bool,
    ) {
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel().with_categorization_enabled(categorization_enabled))
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    fn categorization_on() -> Preferences {
        let preferences = Preferences::default();
        Preferences {
            ai_enabled: true,
            categories_enabled: true,
            ollama_url: Some(OllamaUrl::new("http://mac.local:11434").unwrap()),
            categorization: CategorizationSettings {
                enabled: true,
                ..preferences.categorization
            },
            ..Preferences::default()
        }
    }

    fn prompted_category(name: &str, members: &[CategoryMember]) -> Category {
        category_with_members(name, members).with_ai_prompt(Some(
            CategoryPrompt::new(format!("Videos about {name}"))
                .unwrap()
                .unwrap(),
        ))
    }

    fn summaries_on() -> Preferences {
        let preferences = Preferences::default();
        Preferences {
            ai_enabled: true,
            ollama_url: Some(OllamaUrl::new("http://mac.local:11434").unwrap()),
            summaries: SummarySettings {
                enabled: true,
                ..preferences.summaries
            },
            ..Preferences::default()
        }
    }

    fn summarized_video() -> Video {
        subtitled_video().mark_summary_generated(fixed_timestamp())
    }

    fn subtitled_video() -> Video {
        Video {
            subtitles: Subtitles::done_at(None),
            ..downloaded_video()
        }
    }

    fn my_video() -> Video {
        Video::create(
            YoutubeVideoId::new("yt1").unwrap(),
            "My Video",
            fixed_timestamp(),
        )
    }

    /// Downloaded and numbered in its season folder, with its thumbnail
    /// recorded and its subtitles still Missing, with its chapters and
    /// quality settled, so a sync schedules no chapters fetch or quality check.
    fn downloaded_video() -> Video {
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
                    DOWNLOADED_FILENAME,
                    Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                    None,
                    fixed_timestamp(),
                )
                .with_episode_number(
                    EpisodeNumber::from_columns(2026, 31501).unwrap(),
                    fixed_timestamp(),
                )
                .with_chapters(vec![], fixed_timestamp())
        }
    }

    /// Stores `video` as the only video of `my_channel()`.
    fn save_channel_video(
        video_repository: &SqliteVideoRepository,
        channel_repository: &SqliteChannelRepository,
        channel_video_repository: &SqliteChannelVideoRepository,
        video: &Video,
    ) {
        video_repository.insert(video).unwrap();
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
        video_repository.insert(video).unwrap();
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

    fn ai_verdict(video: &Video, category: &Category, verdict: Verdict) -> VideoCategoryVerdict {
        VideoCategoryVerdict {
            video_id: video.id.clone(),
            category_id: category.id.clone(),
            verdict,
            decided_by: VerdictAuthor::Ai,
            model: Some("qwen3:4b".to_string()),
            reason: Some("A Sicilian game.".to_string()),
            decided_at: fixed_timestamp(),
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
            run_at: fixed_timestamp(),
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
    }

    fn task_for(video: &Video) -> Task {
        Task::SyncVideo {
            video_id: video.id.as_str().to_string(),
        }
    }

    /// The download `my_channel()` wants for `video`.
    fn download_video(video: &Video) -> Task {
        Task::DownloadVideo {
            video_id: video.id.as_str().to_string(),
        }
    }

    fn fetch_thumbnail(video: &Video) -> Task {
        Task::FetchThumbnail {
            video_id: video.id.as_str().to_string(),
        }
    }

    fn fetch_subtitles(video: &Video) -> Task {
        Task::FetchSubtitles {
            video_id: video.id.as_str().to_string(),
        }
    }

    fn categorize_video(video: &Video) -> Task {
        Task::CategorizeVideo {
            video_id: video.id.as_str().to_string(),
        }
    }

    fn summarize_video(video: &Video) -> Task {
        Task::SummarizeVideo {
            video_id: video.id.as_str().to_string(),
        }
    }

    /// Stores `video` as the only video of `my_channel()`.
    fn seed_channel_video(
        db: &TestDatabase,
        video_repository: &SqliteVideoRepository,
        video: &Video,
    ) {
        save_channel_video(
            video_repository,
            &SqliteChannelRepository::new(db.database()),
            &SqliteChannelVideoRepository::new(db.database()),
            video,
        );
    }

    /// A sync whose file, metadata and AI ports no test of it observes; a
    /// downloaded video's file is on disk.
    fn sync_task(
        db: &TestDatabase,
        video_repository: Arc<SqliteVideoRepository>,
        task_repository: Arc<SqliteTaskRepository>,
    ) -> SyncVideoTask {
        SyncVideoTask::new(VideoReconciler::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            task_repository,
            Arc::new(FakeVideoFileRepository::with_listing(vec![
                DOWNLOADED_FILENAME.to_string(),
            ])),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(VideoRelocator::new(
                video_repository,
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            video_category_assigner(db.database()),
            Arc::new(SqlitePreferencesRepository::new(db.database())),
            video_categorizer(db.database()),
        ))
    }

    fn pending_task(id: i64, task: &Task) -> ScheduledTask {
        ScheduledTask {
            id,
            task: task.clone(),
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

    fn run(task: &SyncVideoTask, queued: &Task) -> Result<TaskOutcome, String> {
        task.handle(queued).map_err(|e| e.to_string())
    }

    fn video_category_assigner(database: Database) -> VideoCategoryAssigner {
        VideoCategoryAssigner::new(
            Arc::new(SqliteVideoRepository::new(database.clone())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(database.clone())),
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

    fn video_categorizer(database: Database) -> Arc<VideoCategorizer> {
        Arc::new(VideoCategorizer::new(
            Arc::new(SqlitePreferencesRepository::new(database.clone())),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(database.clone())),
                Arc::new(SqlitePlaylistVideoRepository::new(database.clone())),
                Arc::new(SqliteChannelRepository::new(database.clone())),
                Arc::new(SqlitePlaylistRepository::new(database.clone())),
                "/videos",
            )),
            Arc::new(SqliteCategoryRepository::new(database.clone())),
            Arc::new(SqliteVideoCategoryVerdictRepository::new(database.clone())),
            Arc::new(SqliteVideoSummaryRepository::new(database.clone())),
            Arc::new(FakeLlmServerRepository::failing()),
            video_category_assigner(database.clone()),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(SqliteVideoRepository::new(database)),
        ))
    }
}
