use crate::domain::services::{VideoDownloader, VideoDownloaderApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};

/// Downloads one video via `yt-dlp`, scheduled by
/// the video sync (`SyncVideoTask` via `VideoReconciler`) whenever a video
/// that isn't downloaded yet needs it, or by a reconcile pass.
pub struct DownloadVideoTask {
    video_downloader: VideoDownloader,
}

impl DownloadVideoTask {
    pub fn new(video_downloader: VideoDownloader) -> Self {
        Self { video_downloader }
    }
}

impl TaskHandler for DownloadVideoTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::DownloadVideo { video_id } = task else {
            return Err(unexpected_task(task));
        };
        self.video_downloader
            .download(VideoId::new(video_id.clone())?)
            .map(TaskOutcome::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tasks::log_capture::captured_log_messages;
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::playlist::{Playlist, PlaylistId, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::services::MetadataGenerator;
    use crate::domain::services::VideoOwnerFinder;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::video::DownloadedVideo;
    use crate::domain::video::EpisodeNumber;
    use crate::domain::video::LateDataChecks;
    use crate::domain::video::PlaybackPosition;
    use crate::domain::video::PlaylistEpisodeNumber;
    use crate::domain::video::RetryAttempts;
    use crate::domain::video::Video;
    use crate::domain::video::VideoChapter;
    use crate::domain::video::VideoDownloaded;
    use crate::domain::video::VideoStatus;
    use crate::domain::video::YoutubeVideoId;
    use crate::domain::video_metadata::{VideoMetadata, render_episode_nfo};
    use crate::infrastructure::repositories::filesystem_video_file_repository::{
        FakeVideoFileRepository, FilesystemVideoFileRepository, VideoFileRepository,
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
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::{
        SqliteVideoMetadataRepository, VideoMetadataRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::{
        FailingOnceYoutubeMetadataRepository, FakeYoutubeMetadataRepository, YoutubeMetadata,
        YoutubeMetadataRepository,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::{
        FakeVideoDownloaderRepository, VideoDownloaderRepository, YtDlpVideoDownloaderRepository,
    };
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::staging_area::StagingArea;
    use crate::infrastructure::shared::system_clock::FixedClock;
    #[cfg(unix)]
    use crate::infrastructure::shared::ytdlp::test_support::{FakeYtDlp, unique_temp_dir};
    use chrono::{DateTime, Duration, Utc};
    use rusqlite::Connection;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    #[test]
    fn it_should_save_a_downloaded_video_as_an_episode_in_its_season_folder() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let videos_dir = tempfile::tempdir().unwrap();
        let output_dir = videos_dir.path().join("creators/somechannel");
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .quality_checked_at(fixed_timestamp())
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str())),
                    output_dir
                        .as_path()
                        .join("Season 2026/S2026E031501 - My Video.mp4"),
                ),
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.jpg", video.id.as_str())),
                    output_dir
                        .as_path()
                        .join("Season 2026/S2026E031501 - My Video.jpg"),
                ),
            ]
        );
        assert_eq!(
            std::fs::read_to_string(
                output_dir
                    .as_path()
                    .join("Season 2026/S2026E031501 - My Video.nfo")
            )
            .unwrap(),
            render_episode_nfo(
                &generated_metadata(Some("S2026E031501 - My Video.jpg")),
                &published_episode()
            )
        );
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from(format!(
                "/videos/.downloads/{}",
                video.id.as_str()
            ))]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoDownloaded(VideoDownloaded {
                    video_id: video.id.as_str().to_string(),
                    output_dir: output_dir.as_path().to_string_lossy().to_string(),
                    folder: "Season 2026".to_string(),
                })
            )]
        );
    }

    #[test]
    fn it_should_record_the_downloaded_height() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    height: Some(720),
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                downloaded_height: Some(720),
                quality_checks: LateDataChecks::checked(Some(fixed_timestamp()), 0),
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_settle_high_quality_at_1080() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    height: Some(1080),
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                downloaded_height: Some(1080),
                quality_checks: LateDataChecks::done_at(Some(fixed_timestamp())),
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_settle_quality_of_an_old_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    height: Some(1080),
                    published_at: Some(fixed_timestamp() - Duration::days(30)),
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                downloaded_height: Some(1080),
                quality_checks: LateDataChecks::done_at(Some(fixed_timestamp())),
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2023/S2023E101501 - My Video.mp4",
                        Some("Season 2023/S2023E101501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(
                        EpisodeNumber::from_columns(2023, 101501).unwrap(),
                        fixed_timestamp(),
                    )
            }]
        );
    }

    #[test]
    fn it_should_count_a_degraded_download_as_a_failed_quality_check() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    height: Some(360),
                    sabr_notice: Some(
                        "YouTube may have enabled the SABR-only streaming experiment".to_string(),
                    ),
                    published_at: Some(fixed_timestamp() - Duration::days(30)),
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                downloaded_height: Some(360),
                quality_checks: LateDataChecks::checked(Some(fixed_timestamp()), 1),
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2023/S2023E101501 - My Video.mp4",
                        Some("Season 2023/S2023E101501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(
                        EpisodeNumber::from_columns(2023, 101501).unwrap(),
                        fixed_timestamp(),
                    )
            }]
        );
    }

    #[test]
    fn it_should_leave_quality_unsettled_for_a_young_video_below_its_cap() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel().with_quality(Quality::Mid))
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    height: Some(360),
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                downloaded_height: Some(360),
                quality_checks: LateDataChecks::checked(Some(fixed_timestamp()), 0),
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::Mid,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_settle_quality_at_the_tier_cap() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel().with_quality(Quality::Mid))
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    height: Some(720),
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                downloaded_height: Some(720),
                quality_checks: LateDataChecks::done_at(Some(fixed_timestamp())),
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::Mid,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_record_the_chapters_of_a_downloaded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    chapters: Some(game_chapters()),
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                chapters: Some(game_chapters()),
                chapters_checks: LateDataChecks::done_at(Some(fixed_timestamp())),
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .quality_checked_at(fixed_timestamp())
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_leave_empty_chapters_unsettled_for_a_young_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    chapters: Some(vec![]),
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                chapters: Some(vec![]),
                chapters_checks: LateDataChecks::checked(Some(fixed_timestamp()), 0),
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .quality_checked_at(fixed_timestamp())
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_leave_chapters_unchecked_if_unreadable_on_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    chapters: None,
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                chapters: None,
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .quality_checked_at(fixed_timestamp())
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_write_the_episode_nfo_before_moving_the_media_into_place() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let output_dir = videos_dir.path().join("creators/somechannel");
        let nfo_path = output_dir
            .as_path()
            .join("Season 2026/S2026E031501 - My Video.nfo");
        let nfo_when_media_moved = Arc::new(std::sync::Mutex::new(None));
        let observed = nfo_when_media_moved.clone();
        let observed_nfo_path = nfo_path.clone();
        let video_file_repository = Arc::new(FakeVideoFileRepository::default().with_on_move(
            move |_, to| {
                if to.extension().is_some_and(|ext| ext == "mp4") {
                    *observed.lock().unwrap() = std::fs::read_to_string(&observed_nfo_path).ok();
                }
            },
        ));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            video_file_repository,
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            *nfo_when_media_moved.lock().unwrap(),
            Some(render_episode_nfo(
                &generated_metadata(Some("S2026E031501 - My Video.jpg")),
                &published_episode()
            ))
        );
    }

    #[test]
    fn it_should_number_two_same_day_videos_of_one_show_consecutively() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = SqliteChannelVideoRepository::new(db.database());
        let videos_dir = tempfile::tempdir().unwrap();
        let first = Video::create(
            YoutubeVideoId::new("yt0").unwrap(),
            "First Video",
            fixed_timestamp(),
        )
        .mark_downloaded(
            Quality::High,
            "Season 2026/S2026E031501 - First Video.mp4",
            None,
            None,
            fixed_timestamp(),
        )
        .with_episode_number(published_episode(), fixed_timestamp());
        let video = my_video();
        video_repository.insert(&first).unwrap();
        video_repository.update(&first).unwrap();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                my_channel_id(),
                first.id.clone(),
                1,
                fixed_timestamp(),
            ))
            .unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                2,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                Video {
                    version: 1,
                    ..first
                },
                Video {
                    version: 3,
                    ..video
                        .start_download(fixed_timestamp())
                        .mark_downloaded(
                            Quality::High,
                            "Season 2026/S2026E031502 - My Video.mp4",
                            Some("Season 2026/S2026E031502 - My Video.jpg".to_string()),
                            None,
                            fixed_timestamp(),
                        )
                        .quality_checked_at(fixed_timestamp())
                        .with_episode_number(
                            EpisodeNumber::from_columns(2026, 31502).unwrap(),
                            fixed_timestamp()
                        )
                }
            ]
        );
    }

    #[test]
    fn it_should_reuse_the_number_recorded_by_the_thumbnail_fetch() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let videos_dir = tempfile::tempdir().unwrap();
        let output_dir = videos_dir.path().join("creators/somechannel");
        let prefetched_number = EpisodeNumber::from_columns(2026, 31503).unwrap();
        let video = my_video()
            .with_thumbnail("Season 2026/S2026E031503 - My Video.jpg", fixed_timestamp())
            .with_episode_number(prefetched_number, fixed_timestamp());
        video_repository.insert(&video).unwrap();
        video_repository.update(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031503 - My Video.mp4",
                        Some("Season 2026/S2026E031503 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .quality_checked_at(fixed_timestamp())
            }]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str())),
                    output_dir
                        .as_path()
                        .join("Season 2026/S2026E031503 - My Video.mp4"),
                ),
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.jpg", video.id.as_str())),
                    output_dir
                        .as_path()
                        .join("Season 2026/S2026E031503 - My Video.jpg"),
                ),
            ]
        );
    }

    #[test]
    fn it_should_save_a_playlist_video_under_its_playlist_episode_number() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let videos_dir = tempfile::tempdir().unwrap();
        let output_dir = videos_dir.path().join("music/chill");
        let video = my_video().with_episode_number(
            PlaylistEpisodeNumber::new(7).unwrap().into(),
            fixed_timestamp(),
        );
        video_repository.insert(&video).unwrap();
        SqlitePlaylistRepository::new(db.database())
            .insert(&my_playlist())
            .unwrap();
        SqlitePlaylistVideoRepository::new(db.database())
            .save(&PlaylistVideo::create(
                my_playlist_id(),
                video.id.clone(),
                6,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    published_at: Some(
                        DateTime::parse_from_rfc3339("2014-06-01T10:00:00Z")
                            .unwrap()
                            .to_utc(),
                    ),
                    ..staged_download()
                },
            )),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                quality_checks: LateDataChecks::done_at(Some(fixed_timestamp())),
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 1/S01E007 - My Video.mp4",
                        Some("Season 1/S01E007 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .quality_checked_at(fixed_timestamp())
            }]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str())),
                    output_dir.as_path().join("Season 1/S01E007 - My Video.mp4"),
                ),
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.jpg", video.id.as_str())),
                    output_dir.as_path().join("Season 1/S01E007 - My Video.jpg"),
                ),
            ]
        );
    }

    #[test]
    fn it_should_number_from_the_upload_date_when_no_timestamp_is_reported() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    published_at: Some(
                        DateTime::parse_from_rfc3339("2026-03-16T00:00:00Z")
                            .unwrap()
                            .to_utc(),
                    ),
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                ..video
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031601 - My Video.mp4",
                        Some("Season 2026/S2026E031601 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .quality_checked_at(fixed_timestamp())
                    .with_episode_number(
                        EpisodeNumber::from_columns(2026, 31601).unwrap(),
                        fixed_timestamp()
                    )
            }]
        );
    }

    #[test]
    fn it_should_download_under_its_episode_name_without_metadata() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let videos_dir = tempfile::tempdir().unwrap();
        let output_dir = videos_dir.path().join("creators/somechannel");
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            video_metadata_repository.clone(),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .quality_checked_at(fixed_timestamp())
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str())),
                    output_dir
                        .as_path()
                        .join("Season 2026/S2026E031501 - My Video.mp4"),
                ),
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.jpg", video.id.as_str())),
                    output_dir
                        .as_path()
                        .join("Season 2026/S2026E031501 - My Video.jpg"),
                ),
            ]
        );
        assert_eq!(video_metadata_repository.find(&video.id).unwrap(), None);
        assert_eq!(folder_entries(videos_dir.path()), Vec::<String>::new());
    }

    #[test]
    fn it_should_discard_the_staging_dir_when_the_download_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(false)),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            video_metadata_repository.clone(),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: fixed_timestamp() + Duration::minutes(10),
                reason: format!("yt-dlp failed to download video {}", video.id),
            })
        );
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from(format!(
                "/videos/.downloads/{}",
                video.id.as_str()
            ))]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(video_metadata_repository.find(&video.id).unwrap(), None);
        assert_eq!(folder_entries(videos_dir.path()), Vec::<String>::new());
    }

    #[test]
    fn it_should_retry_a_failed_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(false)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: fixed_timestamp() + Duration::minutes(10),
                reason: format!("yt-dlp failed to download video {}", video.id),
            })
        );
        let failed_id = video.id.clone();
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                ..video.start_download(fixed_timestamp()).mark_errored(
                    format!("yt-dlp failed to download video {}", failed_id),
                    fixed_timestamp()
                )
            }]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_schedule_the_next_attempt_if_the_download_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::with_failed_reason(
                "HTTP Error 503",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: fixed_timestamp() + Duration::minutes(10),
                reason: "HTTP Error 503".to_string(),
            })
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                status: VideoStatus::Retrying,
                download_attempts: RetryAttempts {
                    failed: 1,
                    last_failed_at: Some(fixed_timestamp()),
                },
                download_failure: Some("HTTP Error 503".to_string()),
                updated_at: fixed_timestamp(),
                ..video
            }]
        );
    }

    #[test]
    fn it_should_wait_longer_after_each_failed_attempt() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = retrying_video(1);
        video_repository.insert(&video).unwrap();
        seed_channel_video(&db, &video);
        let task = failing_download_task(&db, video_repository.clone(), "HTTP Error 503");

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: fixed_timestamp() + Duration::hours(1),
                reason: "HTTP Error 503".to_string(),
            })
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                download_attempts: RetryAttempts {
                    failed: 2,
                    last_failed_at: Some(fixed_timestamp()),
                },
                updated_at: fixed_timestamp(),
                ..video
            }]
        );
    }

    #[test]
    fn it_should_record_the_download_error() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::with_failed_reason(
                "HTTP Error 403: Forbidden",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: fixed_timestamp() + Duration::minutes(10),
                reason: "HTTP Error 403: Forbidden".to_string(),
            })
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                ..video
                    .start_download(fixed_timestamp())
                    .mark_errored("HTTP Error 403: Forbidden", fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_still_record_a_degraded_sabr_download_as_downloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    sabr_notice: Some("SABR-only streaming experiment".to_string()),
                    chapters: None,
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                quality_checks: LateDataChecks::checked(Some(fixed_timestamp()), 1),
                ..video
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_fail_and_retry_a_sabr_failure_exactly_as_a_normal_failure() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::with_failed_reason_and_sabr(
                "HTTP Error 403: Forbidden",
                "SABR-only streaming experiment",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: fixed_timestamp() + Duration::minutes(10),
                reason: "HTTP Error 403: Forbidden".to_string(),
            })
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                ..video
                    .start_download(fixed_timestamp())
                    .mark_errored("HTTP Error 403: Forbidden", fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_log_a_sabr_event_on_a_degraded_successful_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with_sabr(
                "SABR-only streaming experiment",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let logs = captured_log_messages(|| {
            let _ = run(&task, &task_for(video.id.as_str()));
        });

        assert!(
            logs.iter().any(
                |message| message.contains("SABR-only streaming experiment reported by yt-dlp")
            )
        );
    }

    #[test]
    fn it_should_log_a_sabr_event_on_a_sabr_download_failure() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::with_failed_reason_and_sabr(
                "HTTP Error 403: Forbidden",
                "SABR-only streaming experiment",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let logs = captured_log_messages(|| {
            let _ = run(&task, &task_for(video.id.as_str()));
        });

        assert!(
            logs.iter().any(
                |message| message.contains("SABR-only streaming experiment reported by yt-dlp")
            )
        );
    }

    #[test]
    fn it_should_not_log_a_sabr_event_on_a_clean_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let logs = captured_log_messages(|| {
            let _ = run(&task, &task_for(video.id.as_str()));
        });

        assert!(
            !logs
                .iter()
                .any(|message| message.contains("SABR-only streaming experiment reported"))
        );
    }

    #[test]
    fn it_should_retry_a_download_that_errored() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::failing(
                "yt-dlp could not be run",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Err("yt-dlp could not be run".to_string()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                ..video
                    .start_download(fixed_timestamp())
                    .mark_errored("yt-dlp could not be run", fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_exclude_a_video_blocked_by_claimed_content() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::with_failed_reason(
                "It was blocked due to the claimed content by Mediatoon. | Video unavailable",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                ..video
                    .start_download(fixed_timestamp())
                    .mark_unavailable(fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_exclude_a_members_only_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::with_failed_reason(
                "Join this channel to get access to members-only content like this video, and other exclusive perks.",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                ..video
                    .start_download(fixed_timestamp())
                    .mark_unavailable(fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_not_exclude_a_bare_video_unavailable() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::with_failed_reason(
                "Video unavailable",
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: fixed_timestamp() + Duration::minutes(10),
                reason: "Video unavailable".to_string(),
            })
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                ..video
                    .start_download(fixed_timestamp())
                    .mark_errored("Video unavailable", fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_skip_if_video_is_gone() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(my_video().id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(*downloader.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_download_at_the_current_quality_into_the_current_show_folder_of_its_owner() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let downloader = Arc::new(FakeVideoDownloaderRepository::succeeding_with(
            staged_download(),
        ));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(
                &my_channel()
                    .with_quality(Quality::Low)
                    .with_path(PlaylistPath::new("creators/renamed").unwrap()),
            )
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            downloader.clone(),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::Low,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .quality_checked_at(fixed_timestamp())
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
        assert_eq!(
            *downloader.calls.lock().unwrap(),
            vec![(
                "https://www.youtube.com/watch?v=yt1".to_string(),
                Quality::Low,
                PathBuf::from(format!("/videos/.downloads/{}", video.id.as_str())),
            )]
        );
        let show_dir = videos_dir.path().join("creators/renamed");
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str())),
                    show_dir.join("Season 2026/S2026E031501 - My Video.mp4"),
                ),
                (
                    PathBuf::from(format!("/videos/.downloads/{}/yt1.jpg", video.id.as_str())),
                    show_dir.join("Season 2026/S2026E031501 - My Video.jpg"),
                ),
            ]
        );
    }

    #[test]
    fn it_should_skip_if_no_playlist_or_channel_holds_the_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            downloader.clone(),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(*downloader.calls.lock().unwrap(), vec![]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_the_download_of_an_already_downloaded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video().mark_downloaded(
            Quality::High,
            "My Video/My Video.mp4",
            None,
            None,
            fixed_timestamp(),
        );
        video_repository.insert(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(*downloader.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_the_download_of_an_excluded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video().mark_unavailable(fixed_timestamp());
        video_repository.insert(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(*downloader.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_the_download_of_an_errored_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video().mark_download_failed("HTTP Error 503", fixed_timestamp());
        video_repository.insert(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(*downloader.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_discard_the_download_when_the_video_was_deleted_meanwhile() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        let deleting_repository = video_repository.clone();
        let deleted_id = video.id.clone();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::succeeding_with(staged_download()).with_on_download(
                    move || {
                        deleting_repository.delete(&deleted_id).unwrap();
                    },
                ),
            ),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![PathBuf::from(format!(
                "/videos/.downloads/{}",
                video.id.as_str()
            ))]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(folder_entries(videos_dir.path()), Vec::<String>::new());
    }

    #[test]
    fn it_should_keep_the_watch_state_recorded_while_downloading() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let watching_repository = video_repository.clone();
        let watched_id = video.id.clone();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::succeeding_with(staged_download()).with_on_download(
                    move || record_watch_state_meanwhile(&watching_repository, &watched_id),
                ),
            ),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 4,
                ..with_watch_state_recorded_meanwhile(
                    video
                        .start_download(fixed_timestamp())
                        .mark_downloaded(
                            Quality::High,
                            "Season 2026/S2026E031501 - My Video.mp4",
                            Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                            None,
                            fixed_timestamp(),
                        )
                        .quality_checked_at(fixed_timestamp())
                        .with_episode_number(published_episode(), fixed_timestamp())
                )
            }]
        );
    }

    #[test]
    fn it_should_keep_the_watch_state_recorded_while_a_failed_download_ran() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        let watching_repository = video_repository.clone();
        let watched_id = video.id.clone();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::new(false).with_on_download(move || {
                    record_watch_state_meanwhile(&watching_repository, &watched_id)
                }),
            ),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: fixed_timestamp() + Duration::minutes(10),
                reason: format!("yt-dlp failed to download video {}", video.id),
            })
        );
        let failed_id = video.id.clone();
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                ..with_watch_state_recorded_meanwhile(
                    video.start_download(fixed_timestamp()).mark_errored(
                        format!("yt-dlp failed to download video {}", failed_id),
                        fixed_timestamp()
                    )
                )
            }]
        );
    }

    #[test]
    fn it_should_keep_the_watch_state_recorded_while_downloading_an_excluded_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        let watching_repository = video_repository.clone();
        let watched_id = video.id.clone();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            Path::new("/videos"),
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::with_failed_reason(
                    "This video has been removed by the uploader.",
                )
                .with_on_download(move || {
                    record_watch_state_meanwhile(&watching_repository, &watched_id)
                }),
            ),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                ..with_watch_state_recorded_meanwhile(
                    video
                        .start_download(fixed_timestamp())
                        .mark_unavailable(fixed_timestamp())
                )
            }]
        );
    }

    #[test]
    fn it_should_keep_the_watch_state_recorded_while_downloading_if_placing_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        let staged_media =
            PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str()));
        video_repository.insert(&video).unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let watching_repository = video_repository.clone();
        let watched_id = video.id.clone();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(
                FakeVideoDownloaderRepository::succeeding_with(staged_download()).with_on_download(
                    move || record_watch_state_meanwhile(&watching_repository, &watched_id),
                ),
            ),
            Arc::new(FakeVideoFileRepository::default().with_failing_move(staged_media.clone())),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Err(format!("permission denied moving {staged_media:?}"))
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 4,
                ..with_watch_state_recorded_meanwhile(
                    video
                        .start_download(fixed_timestamp())
                        .mark_errored(
                            format!("permission denied moving {staged_media:?}"),
                            fixed_timestamp()
                        )
                        .with_episode_number(published_episode(), fixed_timestamp())
                )
            }]
        );
    }

    #[test]
    fn it_should_use_the_sanitized_title_as_filename() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = Video::create(
            YoutubeVideoId::new("yt1").unwrap(),
            "My: Messy / Title?",
            fixed_timestamp(),
        );
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                ..video
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My- Messy - Title-.mp4",
                        Some("Season 2026/S2026E031501 - My- Messy - Title-.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .quality_checked_at(fixed_timestamp())
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_fail_if_invalid_video_id_provided() {
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));

        let result = run(&any_task(downloader.clone()), &task_for(""));

        assert_eq!(result, Err("Video ID must not be empty".to_string()));
        assert_eq!(*downloader.calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_fail_if_handed_another_task() {
        let result = run(
            &any_task(Arc::new(FakeVideoDownloaderRepository::new(true))),
            &Task::UpdateYtdlp,
        );

        assert_eq!(
            result,
            Err("unexpected update_ytdlp task for this handler".to_string())
        );
    }

    #[test]
    fn it_should_record_no_thumbnail_if_none_written() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let videos_dir = tempfile::tempdir().unwrap();
        let output_dir = videos_dir.path().join("creators/somechannel");
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    thumbnail_path: None,
                    ..staged_download()
                },
            )),
            video_file_repository.clone(),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                ..video
                    .clone()
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        None,
                        None,
                        fixed_timestamp(),
                    )
                    .quality_checked_at(fixed_timestamp())
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                PathBuf::from(format!("/videos/.downloads/{}/yt1.mp4", video.id.as_str())),
                output_dir
                    .as_path()
                    .join("Season 2026/S2026E031501 - My Video.mp4"),
            )]
        );
    }

    #[test]
    fn it_should_record_the_duration() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    duration_seconds: Some(223),
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                ..video
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2026/S2026E031501 - My Video.mp4",
                        Some("Season 2026/S2026E031501 - My Video.jpg".to_string()),
                        Some(223),
                        fixed_timestamp(),
                    )
                    .quality_checked_at(fixed_timestamp())
                    .with_episode_number(published_episode(), fixed_timestamp())
            }]
        );
    }

    #[test]
    fn it_should_keep_the_metadata_creation_time_when_regenerated() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let output_dir = videos_dir.path().join("creators/somechannel");
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let earlier = DateTime::<Utc>::from_timestamp(1_600_000_000, 0).unwrap();
        video_metadata_repository
            .save(
                &video.id,
                &VideoMetadata::new(
                    "Stale Title",
                    "Stale plot",
                    "Stale Channel",
                    "Stale Channel",
                    earlier,
                    None,
                    Vec::new(),
                    "yt1",
                    None,
                    "20200913 Stale Title",
                    earlier,
                ),
                &published_episode(),
                &output_dir
                    .as_path()
                    .join("Season 2026/S2026E031501 - My Video.nfo"),
            )
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            video_metadata_repository.clone(),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(VideoMetadata {
                created_at: earlier,
                ..generated_metadata(Some("S2026E031501 - My Video.jpg"))
            })
        );
    }

    #[test]
    fn it_should_save_youtube_metadata() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            video_metadata_repository.clone(),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(generated_metadata(Some("S2026E031501 - My Video.jpg")))
        );
    }

    #[test]
    fn it_should_drop_the_thumbnail_from_the_episode_nfo_if_none_downloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let output_dir = videos_dir.path().join("creators/somechannel");
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                DownloadedVideo {
                    thumbnail_path: None,
                    ..staged_download()
                },
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository {
                metadata: Some(youtube_metadata()),
            }),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            std::fs::read_to_string(
                output_dir
                    .as_path()
                    .join("Season 2026/S2026E031501 - My Video.nfo")
            )
            .unwrap(),
            render_episode_nfo(&generated_metadata(None), &published_episode())
        );
    }

    #[test]
    fn it_should_generate_metadata_if_fetch_fails_only_before_the_download() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let videos_dir = tempfile::tempdir().unwrap();
        let output_dir = videos_dir.path().join("creators/somechannel");
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(video_downloader(
            &db,
            videos_dir.path(),
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::succeeding_with(
                staged_download(),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FailingOnceYoutubeMetadataRepository::new(youtube_metadata())),
            video_metadata_repository.clone(),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(generated_metadata(Some("S2026E031501 - My Video.jpg")))
        );
        assert_eq!(
            std::fs::read_to_string(
                output_dir
                    .as_path()
                    .join("Season 2026/S2026E031501 - My Video.nfo")
            )
            .unwrap(),
            render_episode_nfo(
                &generated_metadata(Some("S2026E031501 - My Video.jpg")),
                &published_episode()
            )
        );
    }

    #[test]
    #[cfg(unix)]
    fn it_should_detect_a_real_thumbnail_file_written_by_yt_dlp_alongside_the_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_root = unique_temp_dir("download-video-task-thumbnail-e2e");
        let fake_yt_dlp = FakeYtDlp::with_downloaded_files("yt1.mp4", &["yt1.mp4", "yt1.jpg"]);
        let video = my_video();
        video_repository.insert(&video).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = DownloadVideoTask::new(VideoDownloader::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                videos_root.to_string_lossy(),
            )),
            Arc::new(YtDlpVideoDownloaderRepository::new(
                fake_yt_dlp.path.clone(),
            )),
            Arc::new(FilesystemVideoFileRepository),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(StagingArea::new(
                Arc::new(FilesystemVideoFileRepository),
                videos_root.to_string_lossy(),
            )),
        ));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 3,
                ..video
                    .start_download(fixed_timestamp())
                    .mark_downloaded(
                        Quality::High,
                        "Season 2023/S2023E111401 - My Video.mp4",
                        Some("Season 2023/S2023E111401 - My Video.jpg".to_string()),
                        None,
                        fixed_timestamp(),
                    )
                    .quality_checked_at(fixed_timestamp())
                    .with_episode_number(
                        EpisodeNumber::from_columns(2023, 111401).unwrap(),
                        fixed_timestamp()
                    )
                    .record_chapters_check(Some(vec![]), fixed_timestamp(), fixed_timestamp())
            }]
        );
        std::fs::remove_dir_all(&videos_root).unwrap();
    }

    /// Builds a downloader around the ports a test seeds, configures or
    /// asserts; the owner lookup (over the channels and playlists a test
    /// seeds in `db`, resolving show folders under `videos_path`), the
    /// playlist membership lookup (only used to pick a metadata sorttitle)
    /// and the clock are ones no test here varies.
    fn video_downloader(
        db: &TestDatabase,
        videos_path: &Path,
        video_repository: Arc<SqliteVideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        youtube_metadata_repository: Arc<dyn YoutubeMetadataRepository>,
        video_metadata_repository: Arc<SqliteVideoMetadataRepository>,
    ) -> VideoDownloader {
        VideoDownloader::new(
            video_repository,
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                videos_path.to_string_lossy(),
            )),
            video_downloader_repository,
            video_file_repository.clone(),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(MetadataGenerator::new(
                youtube_metadata_repository,
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            video_metadata_repository,
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(StagingArea::new(video_file_repository, "/videos")),
        )
    }

    /// A task for tests whose payload is rejected before reaching the
    /// downloader, around `video_downloader_repository` so a test can assert
    /// it was never called. Its repositories sit on an unmigrated in-memory
    /// database, so a payload that wrongly got through would fail loudly
    /// instead of passing.
    fn any_task(
        video_downloader_repository: Arc<FakeVideoDownloaderRepository>,
    ) -> DownloadVideoTask {
        DownloadVideoTask::new(VideoDownloader::new(
            Arc::new(SqliteVideoRepository::new(unused_connection())),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(unused_connection())),
                Arc::new(SqlitePlaylistVideoRepository::new(unused_connection())),
                Arc::new(SqliteChannelRepository::new(unused_connection())),
                Arc::new(SqlitePlaylistRepository::new(unused_connection())),
                "/videos",
            )),
            video_downloader_repository,
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqlitePlaylistVideoRepository::new(unused_connection())),
            Arc::new(MetadataGenerator::new(
                Arc::new(FakeYoutubeMetadataRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteVideoMetadataRepository::new(unused_connection())),
            Arc::new(SqliteEventPublisher::new(
                unused_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(StagingArea::new(
                Arc::new(FakeVideoFileRepository::default()),
                "/videos",
            )),
        ))
    }

    fn unused_connection() -> crate::infrastructure::shared::sqlite_connection::Database {
        crate::infrastructure::shared::sqlite_connection::Database::single(
            Connection::open_in_memory().unwrap(),
        )
    }

    /// `quality_checked_at` for an expected video: its downloaded quality
    /// checked at `at` by the download.
    trait QualityCheckedAt {
        fn quality_checked_at(self, at: DateTime<Utc>) -> Video;
    }

    impl QualityCheckedAt for Video {
        fn quality_checked_at(self, at: DateTime<Utc>) -> Video {
            Video {
                quality_checks: LateDataChecks::checked(Some(at), 0),
                ..self
            }
        }
    }

    fn my_channel_id() -> ChannelHandle {
        ChannelHandle::new("@somechannel").unwrap()
    }

    fn my_playlist_id() -> PlaylistId {
        PlaylistId::new("PL1").unwrap()
    }

    /// A playlist whose show folder is `music/chill` under the videos path.
    fn my_playlist() -> Playlist {
        Playlist::create(
            my_playlist_id(),
            PlaylistName::new("Chill").unwrap(),
            PlaylistPath::new("music/chill").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            false,
            ReconcileInterval::default_interval(),
            fixed_timestamp(),
        )
    }

    /// The channel holding the videos a test downloads, its show folder
    /// `creators/somechannel` under the videos path.
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

    /// A video whose download failed `failed` times, the last a month ago,
    /// so its next attempt is due.
    fn retrying_video(failed: u32) -> Video {
        Video {
            status: VideoStatus::Retrying,
            download_attempts: RetryAttempts {
                failed,
                last_failed_at: Some(fixed_timestamp() - Duration::days(30)),
            },
            download_failure: Some("HTTP Error 503".to_string()),
            ..my_video()
        }
    }

    /// Stores `my_channel()` holding `video`.
    fn seed_channel_video(db: &TestDatabase, video: &Video) {
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
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

    /// A download task whose `yt-dlp` always fails with `reason`.
    fn failing_download_task(
        db: &TestDatabase,
        video_repository: Arc<SqliteVideoRepository>,
        reason: &str,
    ) -> DownloadVideoTask {
        DownloadVideoTask::new(video_downloader(
            db,
            Path::new("/videos"),
            video_repository,
            Arc::new(FakeVideoDownloaderRepository::with_failed_reason(reason)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.database())),
        ))
    }

    fn my_video() -> Video {
        Video::create(
            YoutubeVideoId::new("yt1").unwrap(),
            "My Video",
            fixed_timestamp(),
        )
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

    fn game_chapters() -> Vec<VideoChapter> {
        vec![
            VideoChapter {
                start_seconds: 0,
                end_seconds: 1037,
                title: "Game 1".to_string(),
            },
            VideoChapter {
                start_seconds: 1037,
                end_seconds: 2201,
                title: "Game 2".to_string(),
            },
            VideoChapter {
                start_seconds: 2201,
                end_seconds: 3521,
                title: "Game 3".to_string(),
            },
        ]
    }

    /// A download `yt-dlp` staged for `my_video()`, published on
    /// 2026-03-15, with its converted thumbnail.
    fn staged_download() -> DownloadedVideo {
        DownloadedVideo {
            height: None,
            media_path: PathBuf::from("yt1.mp4"),
            thumbnail_path: Some(PathBuf::from("yt1.jpg")),
            duration_seconds: None,
            published_at: Some(
                DateTime::parse_from_rfc3339("2026-03-15T10:00:00Z")
                    .unwrap()
                    .to_utc(),
            ),
            sabr_notice: None,
            chapters: None,
        }
    }

    /// The first episode of `staged_download()`'s publish day.
    fn published_episode() -> EpisodeNumber {
        EpisodeNumber::from_columns(2026, 31501).unwrap()
    }

    /// The metadata a download of `my_video()` generates from
    /// `youtube_metadata()`, referencing `thumb` when given.
    fn generated_metadata(thumb: Option<&str>) -> VideoMetadata {
        VideoMetadata::new(
            "My Video",
            "A description",
            "My Channel",
            "My Channel",
            fixed_timestamp(),
            None,
            Vec::new(),
            "yt1",
            thumb.map(str::to_string),
            "20231114 My Video",
            fixed_timestamp(),
        )
    }

    fn folder_entries(dir: &std::path::Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect()
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

    fn task_for(video_id: &str) -> Task {
        Task::DownloadVideo {
            video_id: video_id.to_string(),
        }
    }

    /// Stores the video as played and watched, the way marking another copy
    /// watched or playing it does while its download runs.
    fn record_watch_state_meanwhile(video_repository: &SqliteVideoRepository, id: &VideoId) {
        let current = video_repository.find(id).unwrap().unwrap();
        video_repository
            .update(&with_watch_state_recorded_meanwhile(current))
            .unwrap();
    }

    fn with_watch_state_recorded_meanwhile(video: Video) -> Video {
        Video {
            watched_at: Some(watched_timestamp()),
            playback_position: PlaybackPosition::new(42).unwrap(),
            last_played_at: Some(watched_timestamp()),
            ..video
        }
    }

    fn watched_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_100, 0).unwrap()
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn run(task: &DownloadVideoTask, queued: &Task) -> Result<TaskOutcome, String> {
        task.handle(queued).map_err(|e| e.to_string())
    }
}
