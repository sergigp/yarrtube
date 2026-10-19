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
    use crate::domain::services::ChannelAvatarRestorer;
    use crate::domain::services::InternalVideoReconciler;
    use crate::domain::services::MetadataGenerator;
    use crate::domain::services::ShowFileWriter;
    use crate::domain::services::SubtitleFetcher;
    use crate::domain::services::ThumbnailFetcher;
    use crate::domain::services::VideoRelocator;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::domain::video::Video;
    use crate::domain::video::{EpisodeNumber, SubtitleState, VideoId, VideoRecordId, VideoStatus};
    use crate::domain::video_metadata::{VideoMetadata, render_episode_nfo};
    use crate::domain::video_transcript::{
        CaptionLanguage, CaptionSource, CaptionTrack, TranscriptText, VideoTranscript,
    };
    use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FakeChannelAvatarRepository;
    use crate::infrastructure::repositories::filesystem_show_file_repository::FakeShowFileRepository;
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
    use crate::infrastructure::repositories::sqlite_video_transcript_repository::{
        SqliteVideoTranscriptRepository, VideoTranscriptRepository,
    };
    use crate::infrastructure::repositories::youtube_channel_repository::{
        FakeYoutubeChannelRepository, ResolvedChannel, YoutubeChannelRepository,
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
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
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
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
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

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
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

        let result = run(&task, &payload_for("@somechannel"));

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

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
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
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
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
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
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
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
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
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
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
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
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
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
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
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
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
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
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
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
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
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
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
        let event_repository = SqliteEventRepository::new(db.database());
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::unavailable());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
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
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
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
                ..Video::create(VideoId::new("yt1").unwrap(), "One", fixed_timestamp())
            }]
        );
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            vec![ChannelVideo {
                id: 1,
                ..ChannelVideo::create(handle("@somechannel"), video_id, 0, fixed_timestamp())
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
        let event_repository = SqliteEventRepository::new(db.database());
        let show_file_repository = Arc::new(FakeShowFileRepository::failing());
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
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
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
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
                ..ChannelVideo::create(handle("@somechannel"), video_id, 0, fixed_timestamp())
            }]
        );
        assert_eq!(*show_file_repository.show_nfos.lock().unwrap(), vec![]);
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
                    video_id: videos[0].id.as_str().to_string(),
                })
            )]
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
    fn it_should_schedule_a_subtitle_fetch_for_a_downloaded_video_missing_subtitles() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository =
            Arc::new(FakeVideoFileRepository::with_listing(episode_files()));
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

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![fetch_subtitles_task(1, &video.id), next_reconcile(2)]
        );
    }

    #[test]
    fn it_should_not_schedule_a_subtitle_fetch_for_fetched_or_unavailable_subtitles() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository =
            Arc::new(FakeVideoFileRepository::with_listing(episode_files()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let fetched = downloaded_episode().with_subtitles(
            "Season 2023/S2023E111401 - My Video.en.srt",
            fixed_timestamp(),
        );
        let unavailable = Video {
            youtube_id: VideoId::new("yt2").unwrap(),
            ..downloaded_episode().mark_subtitles_unavailable(fixed_timestamp())
        };
        save_channel_video_at(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &fetched,
            0,
        );
        save_channel_video_at(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &unavailable,
            1,
        );
        let task = ReconcileChannelTask::new(channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelVideosRepository::with_videos(vec![
                listed_video("yt1", "My Video", 0),
                listed_video("yt2", "My Video", 1),
            ])),
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
    fn it_should_not_schedule_a_subtitle_fetch_for_a_video_in_the_legacy_layout() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "My Video.mp4".to_string(),
            "My Video.jpg".to_string(),
        ]));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = Video {
            episode_number: Some(recovered_episode()),
            ..downloaded_video("My Video.mp4", Some("My Video.jpg"))
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
                listed_video("yt1", "My Video", 0),
            ])),
            task_repository.clone(),
            video_file_repository.clone(),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![next_reconcile(1)]
        );
    }

    #[test]
    fn it_should_not_schedule_a_subtitle_fetch_for_a_video_not_downloaded() {
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
            thumbnail_filename: Some("Season 2023/S2023E111401 - My Video.jpg".to_string()),
            ..my_video().with_episode_number(recovered_episode(), fixed_timestamp())
        };
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &video,
        );
        task_repository
            .schedule(
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: "high".to_string(),
                    output_dir: "/videos/creators/somechannel".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
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
            task_repository.list_non_completed().unwrap(),
            vec![download_video_task(1, &video.id), next_reconcile(2)]
        );
    }

    #[test]
    fn it_should_not_schedule_a_subtitle_fetch_for_a_video_being_redownloaded() {
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

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![download_video_task(1, &video.id), next_reconcile(2)]
        );
    }

    #[test]
    fn it_should_not_schedule_a_second_subtitle_fetch_if_one_is_queued() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let video_file_repository =
            Arc::new(FakeVideoFileRepository::with_listing(episode_files()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_episode();
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &video,
        );
        task_repository
            .schedule(
                &Task::FetchSubtitles {
                    video_id: video.id.as_str().to_string(),
                    output_dir: "/videos/creators/somechannel".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
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
            task_repository.list_non_completed().unwrap(),
            vec![fetch_subtitles_task(1, &video.id), next_reconcile(2)]
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
        let video = downloaded_episode().with_subtitles(
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

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
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

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
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

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
    }

    #[test]
    fn it_should_clear_the_subtitles_of_a_video_redownloaded() {
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
        let video = downloaded_episode().with_subtitles(
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

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                status: VideoStatus::Pending,
                quality: None,
                filename: None,
                thumbnail_filename: None,
                synced_at: None,
                subtitle_state: SubtitleState::Missing,
                subtitle_filename: None,
                ..video.clone()
            }]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![download_video_task(1, &video.id), next_reconcile(2)]
        );
    }

    #[test]
    fn it_should_reset_a_numbered_video_whose_file_is_missing_without_dropping_its_number() {
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
        let video = downloaded_video(
            "Season 2023/S2023E111401 - My Video.mp4",
            Some("Season 2023/S2023E111401 - My Video.jpg"),
        )
        .with_episode_number(recovered_episode(), fixed_timestamp());
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &video,
        );
        video_repository.update(&video).unwrap();
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
                synced_at: None,
                episode_number: Some(recovered_episode()),
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
    fn it_should_relocate_a_legacy_video_when_its_metadata_is_recovered() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let relocated_files = Arc::new(FakeVideoFileRepository::with_file_exists(true));
        let videos_root = tempfile::tempdir().unwrap();
        let show_dir = videos_root.path().join("creators/somechannel");
        std::fs::create_dir_all(show_dir.join("My Video")).unwrap();
        std::fs::write(show_dir.join("My Video/movie.nfo"), "<movie/>").unwrap();
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_video("My Video/My Video.mp4", Some("My Video/My Video.jpg"));
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &video,
        );
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
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
                    relocated_files.clone(),
                    video_metadata_repository.clone(),
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
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
        ));

        let result = run(&task, &payload_for("@somechannel"));

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
                    show_dir.join("My Video/My Video.mp4"),
                    show_dir.join("Season 2023/S2023E111401 - My Video.mp4"),
                ),
                (
                    show_dir.join("My Video/My Video.jpg"),
                    show_dir.join("Season 2023/S2023E111401 - My Video.jpg"),
                ),
            ]
        );
        assert_eq!(
            *relocated_files.removed_if_empty.lock().unwrap(),
            vec![show_dir.join("My Video")]
        );
        assert!(!show_dir.join("My Video/movie.nfo").exists());
        assert_eq!(
            video_metadata_repository.find(&video.id).unwrap(),
            Some(recovered_metadata())
        );
        assert_eq!(
            std::fs::read_to_string(show_dir.join("Season 2023/S2023E111401 - My Video.nfo"))
                .unwrap(),
            render_episode_nfo(&recovered_metadata(), &recovered_episode())
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![next_reconcile(1)]
        );
    }

    #[test]
    fn it_should_leave_a_legacy_video_in_place_when_its_metadata_is_still_unavailable() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let relocated_files = Arc::new(FakeVideoFileRepository::with_file_exists(true));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let video = downloaded_video("My Video/My Video.mp4", Some("My Video/My Video.jpg"));
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            &video,
        );
        let task = ReconcileChannelTask::new(ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
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
                    Arc::new(FakeYoutubeMetadataRepository::default()),
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
                    video_repository.clone(),
                    relocated_files.clone(),
                    video_metadata_repository.clone(),
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
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
        ));

        let result = run(&task, &payload_for("@somechannel"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(*relocated_files.moved.lock().unwrap(), vec![]);
        assert_eq!(video_metadata_repository.find(&video.id).unwrap(), None);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![next_reconcile(1)]
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
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
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
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
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
        ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
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
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
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
            Arc::new(ChannelAvatarRestorer::new(
                Arc::new(SqliteChannelRepository::new(unused_connection())),
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
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

    /// The metadata reconcile recovers for `my_video()` once relocated.
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

    fn fetch_subtitles_task(id: i64, video_id: &VideoRecordId) -> ScheduledTask {
        pending_task(
            id,
            &Task::FetchSubtitles {
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
