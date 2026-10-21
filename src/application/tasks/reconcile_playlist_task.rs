use crate::domain::playlist::PlaylistId;
use crate::domain::services::{PlaylistVideoReconciler, PlaylistVideoReconcilerApi};
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
    use crate::domain::playlist_video::{VideoAddedToPlaylist, VideoRemovedFromPlaylist};
    use crate::domain::services::InternalVideoReconciler;
    use crate::domain::services::MetadataGenerator;
    use crate::domain::services::ShowFileWriter;
    use crate::domain::services::SubtitleFetcher;
    use crate::domain::services::ThumbnailFetcher;
    use crate::domain::services::VideoRelocator;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::show::ShowKey;
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::domain::video::Video;
    use crate::domain::video::{EpisodeNumber, PlaylistEpisodeNumber, VideoId, VideoRecordId};
    use crate::domain::video_metadata::VideoMetadata;
    use crate::domain::video_transcript::{
        CaptionLanguage, CaptionSource, CaptionTrack, TranscriptText, VideoTranscript,
    };
    use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FakeChannelAvatarRepository;
    use crate::infrastructure::repositories::filesystem_show_file_repository::FakeShowFileRepository;
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
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::{
        SqliteVideoMetadataRepository, VideoMetadataRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_transcript_repository::{
        SqliteVideoTranscriptRepository, VideoTranscriptRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::{
        FakeYoutubeMetadataRepository, YoutubeMetadata,
    };
    use crate::infrastructure::repositories::youtube_playlist_items_repository::{
        FakeYoutubePlaylistItemsRepository, YoutubePlaylistItem,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::show_locks::ShowLocks;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use crate::infrastructure::shared::ytdlp::FetchedThumbnail;
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn it_should_skip_if_playlist_is_gone() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
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
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
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
                ..playlist_episode("vid1", "One", 1)
            }]
        );
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id())
                .unwrap(),
            vec![PlaylistVideo {
                id: 1,
                ..PlaylistVideo::create(playlist_id(), video_id.clone(), 0, fixed_timestamp())
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
                DomainEvent::VideoAddedToPlaylist(VideoAddedToPlaylist {
                    playlist_id: "PL1".to_string(),
                    video_id: video_id.as_str().to_string(),
                })
            )]
        );
    }

    #[test]
    fn it_should_store_a_video_listed_twice_once() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![
                playlist_item("vid1", "One", 0),
                playlist_item("vid1", "One", 2),
            ],
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
                ..playlist_episode("vid1", "One", 1)
            }]
        );
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id())
                .unwrap(),
            vec![PlaylistVideo {
                id: 1,
                ..PlaylistVideo::create(playlist_id(), video_id.clone(), 2, fixed_timestamp())
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
                DomainEvent::VideoAddedToPlaylist(VideoAddedToPlaylist {
                    playlist_id: "PL1".to_string(),
                    video_id: video_id.as_str().to_string(),
                })
            )]
        );
    }

    #[test]
    fn it_should_refresh_title_of_existing_videos() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
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
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
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
                position: 2,
                ..existing_playlist_video
            }]
        );
    }

    #[test]
    fn it_should_number_a_video_added_to_the_playlist_after_the_last_one() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let numbered_playlist = Playlist {
            last_episode: Some(PlaylistEpisodeNumber::new(5).unwrap()),
            ..playlist("PL1")
        };
        playlist_repository.insert(&numbered_playlist).unwrap();
        let existing = playlist_episode("vid1", "One", 5);
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &existing,
            0,
        );
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![
                playlist_item("vid2", "Two", 0),
                playlist_item("vid1", "One", 1),
            ],
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        let videos = video_repository.list().unwrap();
        let added_id = videos[1].id.clone();
        assert_eq!(
            videos,
            vec![
                existing,
                Video {
                    id: added_id,
                    ..playlist_episode("vid2", "Two", 6)
                },
            ]
        );
        assert_eq!(
            playlist_repository.find(&playlist_id()).unwrap(),
            Some(Playlist {
                last_episode: Some(PlaylistEpisodeNumber::new(6).unwrap()),
                ..numbered_playlist
            })
        );
    }

    #[test]
    fn it_should_not_reuse_the_number_of_a_removed_video() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let playlist_after_removal = Playlist {
            last_episode: Some(PlaylistEpisodeNumber::new(6).unwrap()),
            ..playlist("PL1")
        };
        playlist_repository.insert(&playlist_after_removal).unwrap();
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository.clone(),
            video_repository.clone(),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            vec![playlist_item("vid7", "Seven", 0)],
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        let videos = video_repository.list().unwrap();
        let added_id = videos[0].id.clone();
        assert_eq!(
            videos,
            vec![Video {
                id: added_id,
                ..playlist_episode("vid7", "Seven", 7)
            }]
        );
        assert_eq!(
            playlist_repository.find(&playlist_id()).unwrap(),
            Some(Playlist {
                last_episode: Some(PlaylistEpisodeNumber::new(7).unwrap()),
                ..playlist_after_removal
            })
        );
    }

    #[test]
    fn it_should_keep_episode_numbers_when_the_playlist_is_reordered() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "Season 1/S01E001 - One.mp4".to_string(),
            "Season 1/S01E001 - One.jpg".to_string(),
            "Season 1/S01E002 - Two.mp4".to_string(),
            "Season 1/S01E002 - Two.jpg".to_string(),
        ]));
        playlist_repository
            .insert(&Playlist {
                last_episode: Some(PlaylistEpisodeNumber::new(2).unwrap()),
                ..playlist("PL1")
            })
            .unwrap();
        let one = downloaded_playlist_episode("vid1", "One", 1);
        let two = downloaded_playlist_episode("vid2", "Two", 2);
        let one_playlist_video = save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &one,
            0,
        );
        let two_playlist_video = save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &two,
            1,
        );
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![
                playlist_item("vid2", "Two", 0),
                playlist_item("vid1", "One", 1),
            ],
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![one, two]);
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id())
                .unwrap(),
            vec![
                PlaylistVideo {
                    position: 0,
                    ..two_playlist_video
                },
                PlaylistVideo {
                    position: 1,
                    ..one_playlist_video
                },
            ]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_schedule_a_subtitle_fetch_for_a_downloaded_video_missing_subtitles() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "Season 1".to_string(),
            "Season 1/S01E001 - Song.mp4".to_string(),
            "Season 1/S01E001 - Song.jpg".to_string(),
            "Season 1/S01E001 - Song.nfo".to_string(),
        ]));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = downloaded_playlist_episode("vid1", "Song", 1);
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
            vec![playlist_item("vid1", "Song", 0)],
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![
                pending_task(
                    1,
                    &Task::FetchSubtitles {
                        video_id: video.id.as_str().to_string(),
                        output_dir: "/videos/my-playlist".to_string(),
                    },
                    fixed_timestamp(),
                ),
                next_reconcile(2),
            ]
        );
    }

    #[test]
    fn it_should_delete_the_transcript_of_a_removed_video() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = downloaded_playlist_episode("vid1", "Song", 1);
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &video,
            0,
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
        let subtitle_fetcher = Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            video_transcript_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            task_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        ));
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        ));
        let task = ReconcilePlaylistTask::new(PlaylistVideoReconciler::new(
            playlist_repository.clone(),
            video_repository.clone(),
            video_transcript_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository::with_videos(Vec::new())),
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
                video_file_repository.clone(),
                subtitle_fetcher,
                thumbnail_fetcher,
                Arc::new(VideoRelocator::new(
                    Arc::new(SqliteVideoRepository::new(db.database())),
                    Arc::new(FakeVideoFileRepository::default()),
                    Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_remove_videos_gone_from_youtube() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
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
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
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
                    DomainEvent::VideoRemovedFromPlaylist(VideoRemovedFromPlaylist {
                        playlist_id: "PL1".to_string(),
                        video_id: downloaded.id.as_str().to_string(),
                        title: "Downloaded Video".to_string(),
                        filename: Some("Downloaded Video.mp4".to_string()),
                        thumbnail_filename: Some("Downloaded Video.jpg".to_string()),
                        subtitle_filename: None,
                        was_downloaded: true,
                    })
                ),
                pending_event(
                    2,
                    DomainEvent::VideoRemovedFromPlaylist(VideoRemovedFromPlaylist {
                        playlist_id: "PL1".to_string(),
                        video_id: pending.id.as_str().to_string(),
                        title: "Pending Video".to_string(),
                        filename: None,
                        thumbnail_filename: None,
                        subtitle_filename: None,
                        was_downloaded: false,
                    })
                ),
            ]
        );
    }

    #[test]
    fn it_should_always_schedule_the_next_reconcile() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
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
    fn it_should_schedule_the_next_reconcile_after_the_playlists_interval() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        playlist_repository
            .insert(&playlist("PL1").with_reconcile_interval(ReconcileInterval::new(720).unwrap()))
            .unwrap();
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Vec::new(),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::ReconcilePlaylist {
                    playlist_id: "PL1".to_string(),
                },
                fixed_timestamp() + chrono::Duration::minutes(720),
            )]
        );
    }

    #[test]
    fn it_should_not_schedule_the_next_reconcile_if_one_is_already_pending() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let pending_reconcile = Task::ReconcilePlaylist {
            playlist_id: "PL1".to_string(),
        };
        task_repository
            .schedule(
                &pending_reconcile,
                fixed_timestamp() + chrono::Duration::minutes(5),
            )
            .unwrap();
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Vec::new(),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
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
    fn it_should_not_schedule_the_next_reconcile_if_playlist_deleted_during_the_pass() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let task = ReconcilePlaylistTask::new(PlaylistVideoReconciler::new(
            playlist_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(
                FakeYoutubePlaylistItemsRepository::deleting_the_playlist_from(
                    playlist_repository.clone(),
                ),
            ),
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
                Arc::new(SubtitleFetcher::new(
                    video_repository.clone(),
                    Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
                    Arc::new(FakeVideoDownloaderRepository::default()),
                    Arc::new(FakeVideoFileRepository::default()),
                    task_repository.clone(),
                    Arc::new(SqliteEventPublisher::new(
                        db.database(),
                        Arc::new(FixedClock(fixed_timestamp())),
                    )),
                    Arc::new(FixedClock(fixed_timestamp())),
                    "/videos",
                )),
                Arc::new(ThumbnailFetcher::new(
                    video_repository.clone(),
                    Arc::new(FakeVideoDownloaderRepository::default()),
                    Arc::new(FakeVideoFileRepository::default()),
                    task_repository.clone(),
                    Arc::new(FixedClock(fixed_timestamp())),
                    "/videos",
                )),
                Arc::new(VideoRelocator::new(
                    video_repository.clone(),
                    Arc::new(FakeVideoFileRepository::default()),
                    Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(playlist_repository.list().unwrap(), vec![]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[test]
    fn it_should_keep_videos_if_listing_fails() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let existing = my_video();
        let existing_playlist_video = save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &existing,
            0,
        );
        let subtitle_fetcher = Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            task_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        ));
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        ));
        let task = ReconcilePlaylistTask::new(PlaylistVideoReconciler::new(
            playlist_repository,
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            playlist_video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository::failing()),
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
                Arc::new(FakeVideoFileRepository::with_listing(Vec::new())),
                subtitle_fetcher,
                thumbnail_fetcher,
                Arc::new(VideoRelocator::new(
                    Arc::new(SqliteVideoRepository::new(db.database())),
                    Arc::new(FakeVideoFileRepository::default()),
                    Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![existing.clone()]);
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id())
                .unwrap(),
            vec![existing_playlist_video]
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
    fn it_should_redownload_videos_with_missing_file() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
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
    fn it_should_not_sweep_show_files_season_folders_or_episode_files_of_recorded_videos() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "tvshow.nfo".to_string(),
            "poster.jpg".to_string(),
            "Season 2026".to_string(),
            "Season 2026/S2026E031501 - My Video.mp4".to_string(),
            "Season 2026/S2026E031501 - My Video.jpg".to_string(),
            "Season 2026/S2026E031501 - My Video.nfo".to_string(),
            "Season 2026/stray.mp4".to_string(),
        ]));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = episode_video();
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &video,
            0,
        );
        video_repository.update(&video).unwrap();
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
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(
            *video_file_repository.deleted_calls.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/my-playlist"),
                "Season 2026/stray.mp4".to_string()
            )]
        );
    }

    #[test]
    fn it_should_protect_files_sharing_the_base_name_of_an_in_progress_numbered_video() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "Season 2026".to_string(),
            "Season 2026/S2026E031501 - My Video.nfo".to_string(),
        ]));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = my_video()
            .start_download(fixed_timestamp())
            .with_episode_number(
                EpisodeNumber::from_columns(2026, 31501).unwrap(),
                fixed_timestamp(),
            );
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &video,
            0,
        );
        video_repository.update(&video).unwrap();
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
        assert_eq!(video_repository.list().unwrap(), vec![video]);
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_generate_missing_metadata() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let videos_root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(videos_root.path().join("my-playlist/Season 2026")).unwrap();
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = downloaded_video("Season 2026/S2026E031501 - My Video.mp4", None)
            .with_episode_number(
                EpisodeNumber::from_columns(2026, 31501).unwrap(),
                fixed_timestamp(),
            );
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            &video,
            3,
        );
        video_repository.update(&video).unwrap();
        let task = ReconcilePlaylistTask::new(PlaylistVideoReconciler::new(
            playlist_repository,
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            playlist_video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository::with_videos(vec![
                playlist_item("vid1", "My Video", 3),
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
                Arc::new(SubtitleFetcher::new(
                    video_repository.clone(),
                    Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
                    Arc::new(FakeVideoDownloaderRepository::default()),
                    Arc::new(FakeVideoFileRepository::default()),
                    task_repository.clone(),
                    Arc::new(SqliteEventPublisher::new(
                        db.database(),
                        Arc::new(FixedClock(fixed_timestamp())),
                    )),
                    Arc::new(FixedClock(fixed_timestamp())),
                    "/videos",
                )),
                Arc::new(ThumbnailFetcher::new(
                    video_repository.clone(),
                    Arc::new(FakeVideoDownloaderRepository::default()),
                    Arc::new(FakeVideoFileRepository::default()),
                    task_repository.clone(),
                    Arc::new(FixedClock(fixed_timestamp())),
                    "/videos",
                )),
                Arc::new(VideoRelocator::new(
                    Arc::new(SqliteVideoRepository::new(db.database())),
                    Arc::new(FakeVideoFileRepository::default()),
                    Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(FixedClock(fixed_timestamp())),
                videos_root.path().to_string_lossy(),
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
        ));

        let result = run(&task, &payload_for("PL1"));

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
                "vid1",
                None,
                "0003 My Video",
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
                            .join("my-playlist")
                            .to_string_lossy()
                            .to_string(),
                    },
                    fixed_timestamp(),
                ),
                pending_task(
                    2,
                    &Task::FetchSubtitles {
                        video_id: video.id.as_str().to_string(),
                        output_dir: videos_root
                            .path()
                            .join("my-playlist")
                            .to_string_lossy()
                            .to_string(),
                    },
                    fixed_timestamp(),
                ),
                next_reconcile(3),
            ]
        );
    }

    #[test]
    fn it_should_not_fetch_thumbnails_when_adding_new_videos() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_thumbnail_result(Some(fetched_thumbnail())),
        );
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let task = ReconcilePlaylistTask::new(PlaylistVideoReconciler::new(
            playlist_repository,
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            playlist_video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository::with_videos(vec![
                playlist_item("vid1", "One", 0),
                playlist_item("vid2", "Two", 1),
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
                Arc::new(SubtitleFetcher::new(
                    video_repository.clone(),
                    Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
                    video_downloader_repository.clone(),
                    Arc::new(FakeVideoFileRepository::default()),
                    task_repository.clone(),
                    Arc::new(SqliteEventPublisher::new(
                        db.database(),
                        Arc::new(FixedClock(fixed_timestamp())),
                    )),
                    Arc::new(FixedClock(fixed_timestamp())),
                    "/videos",
                )),
                Arc::new(ThumbnailFetcher::new(
                    video_repository.clone(),
                    video_downloader_repository.clone(),
                    Arc::new(FakeVideoFileRepository::default()),
                    task_repository.clone(),
                    Arc::new(FixedClock(fixed_timestamp())),
                    "/videos",
                )),
                Arc::new(VideoRelocator::new(
                    Arc::new(SqliteVideoRepository::new(db.database())),
                    Arc::new(FakeVideoFileRepository::default()),
                    Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        let videos = video_repository.list().unwrap();
        let (first_id, second_id) = (videos[0].id.clone(), videos[1].id.clone());
        assert_eq!(
            videos,
            vec![
                Video {
                    id: first_id.clone(),
                    ..playlist_episode("vid1", "One", 1)
                },
                Video {
                    id: second_id.clone(),
                    ..playlist_episode("vid2", "Two", 2)
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
                    DomainEvent::VideoAddedToPlaylist(VideoAddedToPlaylist {
                        playlist_id: "PL1".to_string(),
                        video_id: first_id.as_str().to_string(),
                    })
                ),
                pending_event(
                    2,
                    DomainEvent::VideoAddedToPlaylist(VideoAddedToPlaylist {
                        playlist_id: "PL1".to_string(),
                        video_id: second_id.as_str().to_string(),
                    })
                ),
            ]
        );
    }

    #[test]
    fn it_should_not_schedule_a_thumbnail_fetch_for_a_video_added_in_the_same_pass() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let playlist_video_repository = Arc::new(SqlitePlaylistVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let task = ReconcilePlaylistTask::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![playlist_item("vid1", "One", 0)],
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
        ));

        let result = run(&task, &payload_for("PL1"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![next_reconcile(1)]
        );
    }

    #[test]
    fn it_should_wait_for_a_move_before_reconciling() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let show_file_repository = Arc::new(FakeShowFileRepository::default());
        let show_locks = Arc::new(ShowLocks::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let task = ReconcilePlaylistTask::new(PlaylistVideoReconciler::new(
            playlist_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(FakeYoutubePlaylistItemsRepository::with_videos(Vec::new())),
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
                Arc::new(SubtitleFetcher::new(
                    video_repository.clone(),
                    Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
                    Arc::new(FakeVideoDownloaderRepository::default()),
                    Arc::new(FakeVideoFileRepository::default()),
                    task_repository.clone(),
                    Arc::new(SqliteEventPublisher::new(
                        db.database(),
                        Arc::new(FixedClock(fixed_timestamp())),
                    )),
                    Arc::new(FixedClock(fixed_timestamp())),
                    "/videos",
                )),
                Arc::new(ThumbnailFetcher::new(
                    video_repository.clone(),
                    Arc::new(FakeVideoDownloaderRepository::default()),
                    Arc::new(FakeVideoFileRepository::default()),
                    task_repository.clone(),
                    Arc::new(FixedClock(fixed_timestamp())),
                    "/videos",
                )),
                Arc::new(VideoRelocator::new(
                    video_repository.clone(),
                    Arc::new(FakeVideoFileRepository::default()),
                    Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                show_file_repository.clone(),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            show_locks.clone(),
        ));

        let result = std::thread::scope(|scope| {
            let moving = show_locks.lock(ShowKey::Playlist(playlist_id()));
            let reconciling = scope.spawn(|| run(&task, &payload_for("PL1")));
            // Gives a reconcile that doesn't wait every chance to run first.
            std::thread::sleep(std::time::Duration::from_millis(100));
            playlist_repository
                .update(
                    &playlist("PL1").with_path(PlaylistPath::new("archive/my-playlist").unwrap()),
                )
                .unwrap();
            drop(moving);
            reconciling.join().unwrap()
        });

        assert_eq!(result, Ok(()));
        assert_eq!(
            *show_file_repository.show_nfos.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/archive/my-playlist"),
                concat!(
                    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                    "<tvshow><title>My Playlist</title><studio>My Playlist</studio>",
                    r#"<uniqueid type="youtube">PL1</uniqueid><status>Continuing</status></tvshow>"#,
                )
                .to_string()
            )]
        );
    }

    /// Builds a reconciler around the repositories and fakes a test seeds,
    /// configures or asserts; the remaining ports (YouTube metadata, video
    /// metadata, thumbnails) are ones these tests don't observe — the ones
    /// that do build the reconciler inline. Events go to `db`'s outbox table.
    fn playlist_video_reconciler(
        db: &TestDatabase,
        playlist_repository: Arc<SqlitePlaylistRepository>,
        video_repository: Arc<SqliteVideoRepository>,
        playlist_video_repository: Arc<SqlitePlaylistVideoRepository>,
        playlist_items: Vec<YoutubePlaylistItem>,
        task_repository: Arc<SqliteTaskRepository>,
        video_file_repository: Arc<FakeVideoFileRepository>,
    ) -> PlaylistVideoReconciler {
        let subtitle_fetcher = Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            task_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        ));
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        ));
        PlaylistVideoReconciler::new(
            playlist_repository,
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            playlist_video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository::with_videos(
                playlist_items,
            )),
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
                subtitle_fetcher,
                thumbnail_fetcher,
                Arc::new(VideoRelocator::new(
                    Arc::new(SqliteVideoRepository::new(db.database())),
                    Arc::new(FakeVideoFileRepository::default()),
                    Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
        )
    }

    /// A task for tests whose payload is rejected before reaching the
    /// reconciler. Its repositories sit on an unmigrated in-memory database,
    /// so a payload that wrongly got through would fail loudly instead of
    /// passing.
    fn any_task() -> ReconcilePlaylistTask {
        let video_repository = Arc::new(SqliteVideoRepository::new(unused_connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            unused_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        ReconcilePlaylistTask::new(PlaylistVideoReconciler::new(
            Arc::new(SqlitePlaylistRepository::new(unused_connection())),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(unused_connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(unused_connection())),
            Arc::new(FakeYoutubePlaylistItemsRepository::with_videos(Vec::new())),
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
                Arc::new(SubtitleFetcher::new(
                    video_repository.clone(),
                    Arc::new(SqliteVideoTranscriptRepository::new(unused_connection())),
                    Arc::new(FakeVideoDownloaderRepository::default()),
                    Arc::new(FakeVideoFileRepository::default()),
                    task_repository.clone(),
                    Arc::new(SqliteEventPublisher::new(
                        unused_connection(),
                        Arc::new(FixedClock(fixed_timestamp())),
                    )),
                    Arc::new(FixedClock(fixed_timestamp())),
                    "/videos",
                )),
                Arc::new(ThumbnailFetcher::new(
                    video_repository,
                    Arc::new(FakeVideoDownloaderRepository::default()),
                    Arc::new(FakeVideoFileRepository::default()),
                    task_repository.clone(),
                    Arc::new(FixedClock(fixed_timestamp())),
                    "/videos",
                )),
                Arc::new(VideoRelocator::new(
                    Arc::new(SqliteVideoRepository::new(unused_connection())),
                    Arc::new(FakeVideoFileRepository::default()),
                    Arc::new(SqliteVideoMetadataRepository::new(unused_connection())),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
        ))
    }

    fn unused_connection() -> crate::infrastructure::shared::sqlite_connection::Database {
        crate::infrastructure::shared::sqlite_connection::Database::single(
            Connection::open_in_memory().unwrap(),
        )
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
            .save(&PlaylistVideo::create(
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
            false,
            ReconcileInterval::default_interval(),
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

    /// `my_video()` downloaded in the TV-show layout as the first episode of
    /// 2026-03-15.
    fn episode_video() -> Video {
        downloaded_video(
            "Season 2026/S2026E031501 - My Video.mp4",
            Some("Season 2026/S2026E031501 - My Video.jpg"),
        )
        .with_episode_number(
            EpisodeNumber::from_columns(2026, 31501).unwrap(),
            fixed_timestamp(),
        )
    }

    /// A newly stored playlist video numbered `episode`.
    fn playlist_episode(youtube_id: &str, title: &str, episode: u32) -> Video {
        Video::create(VideoId::new(youtube_id).unwrap(), title, fixed_timestamp())
            .with_episode_number(
                PlaylistEpisodeNumber::new(episode).unwrap().into(),
                fixed_timestamp(),
            )
    }

    /// A playlist video numbered `episode`, downloaded in the TV-show layout.
    fn downloaded_playlist_episode(youtube_id: &str, title: &str, episode: u32) -> Video {
        let base = format!("Season 1/S01E{episode:03} - {title}");
        playlist_episode(youtube_id, title, episode)
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                format!("{base}.mp4"),
                Some(format!("{base}.jpg")),
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
            path: std::path::PathBuf::from("My Video.jpg"),
            published_at: None,
        }
    }

    fn next_reconcile(id: i64) -> ScheduledTask {
        pending_task(
            id,
            &Task::ReconcilePlaylist {
                playlist_id: "PL1".to_string(),
            },
            fixed_timestamp() + chrono::Duration::minutes(180),
        )
    }

    fn download_video_task(id: i64, video_id: &VideoRecordId) -> ScheduledTask {
        pending_task(
            id,
            &Task::DownloadVideo {
                video_id: video_id.as_str().to_string(),
                quality: "high".to_string(),
                output_dir: "/videos/my-playlist".to_string(),
            },
            fixed_timestamp(),
        )
    }

    fn fetch_thumbnail_task(id: i64, video_id: &VideoRecordId) -> ScheduledTask {
        pending_task(
            id,
            &Task::FetchThumbnail {
                video_id: video_id.as_str().to_string(),
                output_dir: "/videos/my-playlist".to_string(),
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
