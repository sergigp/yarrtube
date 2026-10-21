use crate::domain::playlist::PlaylistId;
use crate::domain::services::{PlaylistVideoReconciler, PlaylistVideoReconcilerApi};
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PlaylistCreatedPayload {
    playlist_id: String,
}

/// Reacts to `PlaylistCreated` by running one reconcile pass for the new
/// playlist, as that event's own processing (not as a separately scheduled
/// task).
pub struct ReconcileOnPlaylistCreated {
    playlist_video_reconciler: PlaylistVideoReconciler,
}

impl ReconcileOnPlaylistCreated {
    pub fn new(playlist_video_reconciler: PlaylistVideoReconciler) -> Self {
        Self {
            playlist_video_reconciler,
        }
    }
}

impl EventSubscriber for ReconcileOnPlaylistCreated {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: PlaylistCreatedPayload = serde_json::from_str(payload)?;
        let Ok(playlist_id) = PlaylistId::new(payload.playlist_id) else {
            return Ok(());
        };
        self.playlist_video_reconciler
            .initial_reconcile(playlist_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::services::InternalVideoReconciler;
    use crate::domain::services::MetadataGenerator;
    use crate::domain::services::ShowFileWriter;
    use crate::domain::services::SubtitleFetcher;
    use crate::domain::services::ThumbnailFetcher;
    use crate::domain::services::VideoRelocator;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::task::{ScheduledTask, Task, TaskStatus};
    use crate::domain::video::{PlaylistEpisodeNumber, Video, VideoId, VideoRecordId};
    use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FakeChannelAvatarRepository;
    use crate::infrastructure::repositories::filesystem_show_file_repository::FakeShowFileRepository;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        PlaylistRepository, SqlitePlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::SqliteVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::repositories::sqlite_video_transcript_repository::SqliteVideoTranscriptRepository;
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
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
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn it_should_reconcile_the_playlist() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let subscriber = ReconcileOnPlaylistCreated::new(playlist_video_reconciler(
            &db,
            playlist_repository,
            task_repository.clone(),
        ));

        let result = handle(&subscriber, r#"{"playlist_id": "PL1"}"#);

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::ReconcilePlaylist {
                    playlist_id: "PL1".to_string(),
                },
                fixed_timestamp() + chrono::Duration::minutes(180),
            )]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_number_new_playlist_videos_in_playlist_order() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let subscriber = ReconcileOnPlaylistCreated::new(PlaylistVideoReconciler::new(
            playlist_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(FakeYoutubePlaylistItemsRepository::with_videos(vec![
                playlist_item("vid3", "Third", 2),
                playlist_item("vid1", "First", 0),
                playlist_item("vid2", "Second", 1),
            ])),
            event_publisher(&db),
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

        let result = handle(&subscriber, r#"{"playlist_id": "PL1"}"#);

        assert_eq!(result, Ok(()));
        let videos = video_repository.list().unwrap();
        let ids: Vec<VideoRecordId> = videos.iter().map(|video| video.id.clone()).collect();
        assert_eq!(
            videos,
            vec![
                Video {
                    id: ids[0].clone(),
                    ..playlist_episode("vid1", "First", 1)
                },
                Video {
                    id: ids[1].clone(),
                    ..playlist_episode("vid2", "Second", 2)
                },
                Video {
                    id: ids[2].clone(),
                    ..playlist_episode("vid3", "Third", 3)
                },
            ]
        );
        assert_eq!(
            playlist_repository
                .find(&PlaylistId::new("PL1").unwrap())
                .unwrap(),
            Some(Playlist {
                last_episode: Some(PlaylistEpisodeNumber::new(3).unwrap()),
                ..playlist("PL1")
            })
        );
    }

    #[test]
    fn it_should_write_the_show_nfo_without_a_poster_for_a_playlist() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let show_file_repository = Arc::new(FakeShowFileRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let subscriber = ReconcileOnPlaylistCreated::new(PlaylistVideoReconciler::new(
            playlist_repository,
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(FakeYoutubePlaylistItemsRepository::with_videos(Vec::new())),
            event_publisher(&db),
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
            Arc::new(ShowLocks::default()),
        ));

        let result = handle(&subscriber, r#"{"playlist_id": "PL1"}"#);

        assert_eq!(result, Ok(()));
        assert_eq!(
            *show_file_repository.show_nfos.lock().unwrap(),
            vec![(
                PathBuf::from("/videos/my-playlist"),
                concat!(
                    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                    "<tvshow><title>My Playlist</title><studio>My Playlist</studio>",
                    r#"<uniqueid type="youtube">PL1</uniqueid><status>Continuing</status></tvshow>"#,
                )
                .to_string()
            )]
        );
        assert_eq!(*show_file_repository.posters.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_fail_if_listing_fails() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let subscriber = ReconcileOnPlaylistCreated::new(PlaylistVideoReconciler::new(
            playlist_repository,
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(FakeYoutubePlaylistItemsRepository::failing()),
            event_publisher(&db),
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

        let result = handle(&subscriber, r#"{"playlist_id": "PL1"}"#);

        assert_eq!(result, Err("failed to list playlist items".to_string()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_if_invalid_playlist_id_provided() {
        let result = handle(&any_subscriber(), r#"{"playlist_id": ""}"#);

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn it_should_skip_if_playlist_is_gone() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        let subscriber = ReconcileOnPlaylistCreated::new(playlist_video_reconciler(
            &db,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            task_repository.clone(),
        ));

        let result = handle(&subscriber, r#"{"playlist_id": "PL404"}"#);

        assert_eq!(result, Ok(()));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    /// Builds a reconciler around the repositories a test seeds and asserts;
    /// the remaining ports (YouTube, metadata, files, thumbnails) are ones no
    /// test here observes. Events go to `db`'s outbox table.
    fn playlist_video_reconciler(
        db: &TestDatabase,
        playlist_repository: Arc<SqlitePlaylistRepository>,
        task_repository: Arc<SqliteTaskRepository>,
    ) -> PlaylistVideoReconciler {
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
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
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(FakeYoutubePlaylistItemsRepository::with_videos(Vec::new())),
            event_publisher(db),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository,
                Arc::new(MetadataGenerator::new(
                    Arc::new(FakeYoutubeMetadataRepository::default()),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                task_repository,
                Arc::new(FakeVideoFileRepository::default()),
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

    /// A subscriber for tests whose payload is rejected before reaching the
    /// reconciler. Its repositories sit on an unmigrated in-memory database,
    /// so a payload that wrongly got through would fail loudly instead of
    /// passing.
    fn any_subscriber() -> ReconcileOnPlaylistCreated {
        let video_repository = Arc::new(SqliteVideoRepository::new(unused_connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            unused_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        ReconcileOnPlaylistCreated::new(PlaylistVideoReconciler::new(
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

    fn playlist_item(video_id: &str, title: &str, position: i64) -> YoutubePlaylistItem {
        YoutubePlaylistItem {
            video_id: video_id.to_string(),
            title: title.to_string(),
            position,
        }
    }

    /// A newly stored playlist video numbered `episode`.
    fn playlist_episode(youtube_id: &str, title: &str, episode: u32) -> Video {
        Video::create(VideoId::new(youtube_id).unwrap(), title, fixed_timestamp())
            .with_episode_number(
                PlaylistEpisodeNumber::new(episode).unwrap().into(),
                fixed_timestamp(),
            )
    }

    fn event_publisher(db: &TestDatabase) -> Arc<SqliteEventPublisher> {
        Arc::new(SqliteEventPublisher::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ))
    }

    fn unused_connection() -> crate::infrastructure::shared::sqlite_connection::Database {
        crate::infrastructure::shared::sqlite_connection::Database::single(
            Connection::open_in_memory().unwrap(),
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

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn handle(subscriber: &ReconcileOnPlaylistCreated, payload: &str) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
