use crate::domain::services::{SubtitleFetcher, SubtitleFetcherApi};
use crate::domain::task::{Task, TaskOutcome};
use crate::domain::video::VideoId;
use crate::infrastructure::repositories::task_handler::{TaskHandler, unexpected_task};
use std::sync::Arc;

/// Fetches one downloaded video's subtitles, scheduled by
/// the video sync (`SyncVideoTask` via `VideoReconciler`) after a download
/// and by the reconcilers' missing-subtitles recovery. Best-effort: always succeeds, so
/// the queue never retries it; the next reconcile pass reschedules it instead.
pub struct FetchSubtitlesTask {
    subtitle_fetcher: Arc<SubtitleFetcher>,
}

impl FetchSubtitlesTask {
    pub fn new(subtitle_fetcher: Arc<SubtitleFetcher>) -> Self {
        Self { subtitle_fetcher }
    }
}

impl TaskHandler for FetchSubtitlesTask {
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome> {
        let Task::FetchSubtitles { video_id } = task else {
            return Err(unexpected_task(task));
        };
        self.subtitle_fetcher
            .fetch(&VideoId::new(video_id.clone())?)
            .map(TaskOutcome::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::services::VideoOwnerFinder;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::video::Subtitles;
    use crate::domain::video::{
        EpisodeNumber, PlaybackPosition, RetryAttempts, StepState, StepStatus, Video,
        VideoSubtitlesFetched, YoutubeVideoId,
    };
    use crate::domain::video_transcript::{
        CaptionLanguage, CaptionProbe, CaptionSource, CaptionTracks, FetchedSubtitles,
        TranscriptText, VideoTranscript,
    };
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, SqliteChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, SqliteChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_transcript_repository::{
        SqliteVideoTranscriptRepository, VideoTranscriptRepository,
    };
    use crate::infrastructure::repositories::youtube_caption_repository::FakeCaptionRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::staging_area::StagingArea;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Duration, Utc};
    use std::path::PathBuf;

    #[test]
    fn it_should_fetch_the_original_language_subtitles() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(CaptionTracks {
                original_language: Some("en".to_string()),
                manual: vec!["en".to_string(), "fr".to_string()],
                automatic: vec!["en-orig".to_string(), "en".to_string()],
            })))
            .with_subtitles(Ok(fetched_subtitles("yt1.en.srt"))),
        );
        let video = downloaded_video();
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                updated_at: now(),
                subtitles: Subtitles {
                    filename: Some("Season 2026/S2026E031501 - Title.en.srt".to_string()),
                    ..Subtitles::done_at(Some(now()))
                },
                ..video.clone()
            }]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                staging_dir(&video).join("yt1.en.srt"),
                PathBuf::from("/videos/my-channel/Season 2026/S2026E031501 - Title.en.srt"),
            )]
        );
        assert_eq!(
            video_transcript_repository.find(&video.id).unwrap(),
            Some(VideoTranscript {
                video_id: video.id.clone(),
                language: CaptionLanguage::new("en").unwrap(),
                source: CaptionSource::Manual,
                text: TranscriptText::new("Hello there\nGeneral Kenobi"),
                created_at: now(),
            })
        );
    }

    #[test]
    fn it_should_name_the_subtitles_after_the_downloaded_file() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Ok(fetched_subtitles("yt1.en.srt"))),
        );
        let video = Video {
            title: "New Title".to_string(),
            ..downloaded_video()
                .map_file(|file| file.with_filename("Season 2026/S2026E031501 - Old Title.mp4"))
        };
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..video
                    .clone()
                    .mark_subtitles_fetched("Season 2026/S2026E031501 - Old Title.en.srt", now())
            }]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                staging_dir(&video).join("yt1.en.srt"),
                PathBuf::from("/videos/my-channel/Season 2026/S2026E031501 - Old Title.en.srt"),
            )]
        );
    }

    #[test]
    fn it_should_record_auto_generated_captions_as_automatic() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(CaptionTracks {
                original_language: Some("es".to_string()),
                manual: vec!["en".to_string()],
                automatic: vec!["es-orig".to_string(), "en".to_string()],
            })))
            .with_subtitles(Ok(fetched_subtitles("yt1.es-orig.srt"))),
        );
        let video = downloaded_video();
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..video
                    .clone()
                    .mark_subtitles_fetched("Season 2026/S2026E031501 - Title.es.srt", now())
            }]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                staging_dir(&video).join("yt1.es-orig.srt"),
                PathBuf::from("/videos/my-channel/Season 2026/S2026E031501 - Title.es.srt"),
            )]
        );
        assert_eq!(
            video_transcript_repository.find(&video.id).unwrap(),
            Some(VideoTranscript {
                video_id: video.id.clone(),
                language: CaptionLanguage::new("es").unwrap(),
                source: CaptionSource::Automatic,
                text: TranscriptText::new("Hello there\nGeneral Kenobi"),
                created_at: now(),
            })
        );
    }

    #[test]
    fn it_should_name_a_regional_track_with_its_base_language() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(CaptionTracks {
                original_language: Some("es-US".to_string()),
                manual: vec!["es-US".to_string()],
                automatic: vec!["es-orig".to_string()],
            })))
            .with_subtitles(Ok(fetched_subtitles("yt1.es-US.srt"))),
        );
        let video = downloaded_video();
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..video
                    .clone()
                    .mark_subtitles_fetched("Season 2026/S2026E031501 - Title.es.srt", now())
            }]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                staging_dir(&video).join("yt1.es-US.srt"),
                PathBuf::from("/videos/my-channel/Season 2026/S2026E031501 - Title.es.srt"),
            )]
        );
        assert_eq!(
            video_transcript_repository.find(&video.id).unwrap(),
            Some(VideoTranscript {
                video_id: video.id.clone(),
                language: CaptionLanguage::new("es-US").unwrap(),
                source: CaptionSource::Manual,
                text: TranscriptText::new("Hello there\nGeneral Kenobi"),
                created_at: now(),
            })
        );
    }

    #[test]
    fn it_should_mark_old_subtitles_unavailable_without_captions() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(FakeCaptionRepository::with_probe(Ok(caption_probe(
            translations_only(),
        ))));
        let video = Video {
            subtitles: Subtitles::checked(Some(now() - Duration::days(8)), 0),
            ..downloaded_video()
        };
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                updated_at: now(),
                subtitles: Subtitles {
                    filename: video.subtitles.filename.clone(),
                    ..Subtitles::unavailable_at(Some(now()))
                },
                ..video.clone()
            }]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_keep_young_subtitles_pending_after_a_conclusive_check() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(FakeCaptionRepository::with_probe(Ok(CaptionProbe {
            published_at: Some(now() - Duration::days(2)),
            ..caption_probe(translations_only())
        })));
        let video = Video {
            subtitles: Subtitles::checked(Some(now() - Duration::days(1)), 4),
            ..downloaded_video()
        };
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                updated_at: now(),
                subtitles: Subtitles {
                    filename: video.subtitles.filename.clone(),
                    ..Subtitles::checked(Some(now()), 0)
                },
                ..video.clone()
            }]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_mark_subtitles_unavailable_on_first_check_of_an_old_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(FakeCaptionRepository::with_probe(Ok(caption_probe(
            translations_only(),
        ))));
        let video = downloaded_video();
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                updated_at: now(),
                subtitles: Subtitles {
                    filename: video.subtitles.filename.clone(),
                    ..Subtitles::unavailable_at(Some(now()))
                },
                ..video.clone()
            }]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_fall_back_to_the_creation_time_when_no_publish_time_is_reported() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(FakeCaptionRepository::with_probe(Ok(CaptionProbe {
            published_at: None,
            ..caption_probe(translations_only())
        })));
        let video = Video {
            subtitles: Subtitles::checked(Some(now() - Duration::days(8)), 0),
            ..downloaded_video()
        };
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                updated_at: now(),
                subtitles: Subtitles {
                    filename: video.subtitles.filename.clone(),
                    ..Subtitles::unavailable_at(Some(now()))
                },
                ..video.clone()
            }]
        );
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_count_a_failed_attempt_if_the_probe_warns() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(FakeCaptionRepository::with_probe(Ok(CaptionProbe {
            warnings: vec!["WARNING: [youtube] yt1: Some subtitles may be missing".to_string()],
            ..caption_probe(translations_only())
        })));
        let video = downloaded_video();
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: now() + Duration::minutes(10),
                reason: "WARNING: [youtube] yt1: Some subtitles may be missing".to_string(),
            })
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                updated_at: now(),
                subtitles: Subtitles {
                    filename: video.subtitles.filename.clone(),
                    ..Subtitles::checked(Some(now()), 1)
                },
                ..video.clone()
            }]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_schedule_the_next_check_if_the_subtitle_check_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(FakeCaptionRepository::with_probe(Err(anyhow::anyhow!(
            "HTTP Error 429: Too Many Requests"
        ))));
        let video = downloaded_video();
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: now() + Duration::minutes(10),
                reason: "HTTP Error 429: Too Many Requests".to_string(),
            })
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                updated_at: now(),
                subtitles: Subtitles {
                    filename: video.subtitles.filename.clone(),
                    ..Subtitles::checked(Some(now()), 1)
                },
                ..video.clone()
            }]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_count_a_failed_attempt_if_the_subtitles_download_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Err(anyhow::anyhow!("HTTP Error 429: Too Many Requests"))),
        );
        let video = downloaded_video();
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: now() + Duration::minutes(10),
                reason: "HTTP Error 429: Too Many Requests".to_string(),
            })
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                updated_at: now(),
                subtitles: Subtitles {
                    filename: video.subtitles.filename.clone(),
                    ..Subtitles::checked(Some(now()), 1)
                },
                ..video.clone()
            }]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_keep_retrying_failed_checks_of_a_young_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(FakeCaptionRepository::with_probe(Err(anyhow::anyhow!(
            "HTTP Error 429: Too Many Requests"
        ))));
        let video = Video {
            created_at: now() - Duration::days(2),
            subtitles: Subtitles::checked(Some(now() - Duration::days(1)), 5),
            ..downloaded_video()
        };
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: now() + Duration::hours(24),
                reason: "HTTP Error 429: Too Many Requests".to_string(),
            })
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                updated_at: now(),
                subtitles: Subtitles {
                    filename: video.subtitles.filename.clone(),
                    ..Subtitles::checked(Some(now()), 6)
                },
                ..video.clone()
            }]
        );
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_fail_the_subtitles_after_the_eleventh_failed_check() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let caption_repository = Arc::new(FakeCaptionRepository::with_probe(Err(anyhow::anyhow!(
            "HTTP Error 429: Too Many Requests"
        ))));
        let video = Video {
            created_at: now() - Duration::days(2),
            subtitles: Subtitles::checked(Some(now() - Duration::days(1)), 10),
            ..downloaded_video()
        };
        video_repository.insert(&video).unwrap();
        seed_channel_video(&db, &video);
        let task = subtitles_task(&db, video_repository.clone(), caption_repository);

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                updated_at: now(),
                subtitles: Subtitles {
                    state: StepState {
                        status: StepStatus::Failed,
                        attempts: RetryAttempts {
                            failed: 11,
                            last_failed_at: Some(now()),
                        },
                    },
                    last_checked_at: Some(now()),
                    filename: video.subtitles.filename.clone()
                },
                ..video
            }]
        );
    }

    #[test]
    fn it_should_skip_if_video_no_longer_exists() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(FakeCaptionRepository::default());

        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for("missing"));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(*caption_repository.calls.lock().unwrap(), 0);
    }

    #[test]
    fn it_should_skip_if_no_playlist_or_channel_holds_the_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Ok(fetched_subtitles("yt1.en.srt"))),
        );
        let video = downloaded_video();
        video_repository.insert(&video).unwrap();
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(*caption_repository.calls.lock().unwrap(), 0);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_discard_the_subtitles_if_the_video_was_deleted_meanwhile() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let event_repository = SqliteEventRepository::new(db.database());
        let video = downloaded_video();
        video_repository.insert(&video).unwrap();
        let deleting_repository = video_repository.clone();
        let deleted_id = video.id.clone();
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Ok(fetched_subtitles("yt1.en.srt")))
                .with_on_fetch_subtitles(move || {
                    deleting_repository.delete(&deleted_id).unwrap();
                }),
        );
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository,
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![staging_dir(&video)]
        );
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_discard_the_subtitles_if_the_video_was_reset_for_redownload_meanwhile() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let event_repository = SqliteEventRepository::new(db.database());
        let video = downloaded_video();
        video_repository.insert(&video).unwrap();
        let reset = video.clone().reset_for_redownload(now());
        let resetting_repository = video_repository.clone();
        let reset_video = reset.clone();
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Ok(fetched_subtitles("yt1.en.srt")))
                .with_on_fetch_subtitles(move || {
                    resetting_repository.update(&reset_video).unwrap();
                }),
        );
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository,
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..reset
            }]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![staging_dir(&video)]
        );
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_discard_the_subtitles_if_the_video_was_redownloaded_under_another_name_meanwhile()
    {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let event_repository = SqliteEventRepository::new(db.database());
        let video = downloaded_video();
        video_repository.insert(&video).unwrap();
        let redownloaded = video
            .clone()
            .map_file(|file| file.with_filename("Season 2026/S2026E031501 - New Title.mp4"));
        let redownloading_repository = video_repository.clone();
        let redownloaded_video = redownloaded.clone();
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Ok(fetched_subtitles("yt1.en.srt")))
                .with_on_fetch_subtitles(move || {
                    redownloading_repository
                        .update(&redownloaded_video)
                        .unwrap();
                }),
        );
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository,
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                ..redownloaded
            }]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![staging_dir(&video)]
        );
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_if_subtitles_already_fetched() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Ok(fetched_subtitles("yt1.en.srt"))),
        );
        let fetched = downloaded_video()
            .mark_subtitles_fetched("Season 2026/S2026E031501 - Title.en.srt", fixed_timestamp());
        let unavailable = downloaded_video().mark_subtitles_unavailable(fixed_timestamp());
        video_repository.insert(&fetched).unwrap();
        video_repository.insert(&unavailable).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                fetched.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                unavailable.id.clone(),
                1,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let results = [
            run(&task, &task_for(fetched.id.as_str())),
            run(&task, &task_for(unavailable.id.as_str())),
        ];

        assert_eq!(results, [Ok(TaskOutcome::Done), Ok(TaskOutcome::Done)]);
        assert_eq!(video_repository.list().unwrap(), vec![fetched, unavailable]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(*caption_repository.calls.lock().unwrap(), 0);
    }

    #[test]
    fn it_should_skip_a_video_that_is_not_downloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Ok(fetched_subtitles("yt1.en.srt"))),
        );
        let pending = Video::create(
            YoutubeVideoId::new("yt1").unwrap(),
            "Title",
            fixed_timestamp(),
        )
        .with_episode_number(
            EpisodeNumber::from_columns(2026, 31501).unwrap(),
            fixed_timestamp(),
        );
        let legacy = Video::create(
            YoutubeVideoId::new("yt2").unwrap(),
            "Old",
            fixed_timestamp(),
        )
        .mark_downloaded(Quality::High, "Old/Old.mp4", None, None, fixed_timestamp());
        video_repository.insert(&pending).unwrap();
        video_repository.insert(&legacy).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                pending.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                legacy.id.clone(),
                1,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let results = [
            run(&task, &task_for(pending.id.as_str())),
            run(&task, &task_for(legacy.id.as_str())),
        ];

        assert_eq!(results, [Ok(TaskOutcome::Done), Ok(TaskOutcome::Done)]);
        assert_eq!(video_repository.list().unwrap(), vec![pending, legacy]);
        assert_eq!(*caption_repository.calls.lock().unwrap(), 0);
    }

    #[test]
    fn it_should_skip_a_numbered_video_still_in_the_legacy_layout() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Ok(fetched_subtitles("yt1.en.srt"))),
        );
        let in_folder = Video::create(
            YoutubeVideoId::new("yt1").unwrap(),
            "Old",
            fixed_timestamp(),
        )
        .with_episode_number(
            EpisodeNumber::from_columns(2026, 31501).unwrap(),
            fixed_timestamp(),
        )
        .mark_downloaded(Quality::High, "Old/Old.mp4", None, None, fixed_timestamp());
        let flat = Video::create(
            YoutubeVideoId::new("yt2").unwrap(),
            "Flat",
            fixed_timestamp(),
        )
        .with_episode_number(
            EpisodeNumber::from_columns(2026, 31502).unwrap(),
            fixed_timestamp(),
        )
        .mark_downloaded(Quality::High, "Flat.mp4", None, None, fixed_timestamp());
        video_repository.insert(&in_folder).unwrap();
        video_repository.insert(&flat).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                in_folder.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                flat.id.clone(),
                1,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let results = [
            run(&task, &task_for(in_folder.id.as_str())),
            run(&task, &task_for(flat.id.as_str())),
        ];

        assert_eq!(results, [Ok(TaskOutcome::Done), Ok(TaskOutcome::Done)]);
        assert_eq!(video_repository.list().unwrap(), vec![in_folder, flat]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(*caption_repository.calls.lock().unwrap(), 0);
    }

    #[test]
    fn it_should_keep_watch_progress_recorded_while_fetching() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video = downloaded_video();
        let progressed = video.clone().update_watch_state(
            PlaybackPosition::new(30).unwrap(),
            None,
            false,
            now(),
        );
        let video_file_repository = Arc::new(FakeVideoFileRepository::default().with_on_move({
            let video_repository = video_repository.clone();
            let progressed = progressed.clone();
            move |_, _| video_repository.update(&progressed).unwrap()
        }));
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Ok(fetched_subtitles("yt1.en.srt"))),
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                ..progressed
                    .mark_subtitles_fetched("Season 2026/S2026E031501 - Title.en.srt", now())
            }]
        );
    }

    #[test]
    fn it_should_keep_download_progress_while_recording_subtitles() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video = downloaded_video();
        let redownloaded = video.clone().mark_downloaded(
            Quality::Low,
            "Season 2026/S2026E031501 - Title.mp4",
            Some("Season 2026/S2026E031501 - Title.jpg".to_string()),
            Some(120),
            fixed_timestamp(),
        );
        let video_file_repository = Arc::new(FakeVideoFileRepository::default().with_on_move({
            let video_repository = video_repository.clone();
            let redownloaded = redownloaded.clone();
            move |_, _| video_repository.update(&redownloaded).unwrap()
        }));
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Ok(fetched_subtitles("yt1.en.srt"))),
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 2,
                ..redownloaded
                    .mark_subtitles_fetched("Season 2026/S2026E031501 - Title.en.srt", now())
            }]
        );
    }

    #[test]
    fn it_should_discard_the_staging_dir() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Ok(fetched_subtitles("yt1.en.srt"))),
        );
        let fetched = downloaded_video();
        let failed = downloaded_video();
        video_repository.insert(&fetched).unwrap();
        video_repository.insert(&failed).unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                fetched.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                my_channel_id(),
                failed.id.clone(),
                1,
                fixed_timestamp(),
            ))
            .unwrap();
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        // The fake's probe is used up by the first fetch: the second one fails.
        let results = [
            run(&task, &task_for(fetched.id.as_str())),
            run(&task, &task_for(failed.id.as_str())),
        ];

        assert_eq!(
            results,
            [
                Ok(TaskOutcome::Done),
                Ok(TaskOutcome::RunAgainAt {
                    run_at: now() + Duration::minutes(10),
                    reason: "no fake caption probe configured".to_string(),
                }),
            ]
        );
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![staging_dir(&fetched), staging_dir(&failed)]
        );
    }

    #[test]
    fn it_should_publish_that_subtitles_were_fetched() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Ok(fetched_subtitles("yt1.en.srt"))),
        );
        let video = downloaded_video();
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            caption_repository,
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(
                Arc::new(FakeVideoFileRepository::default()),
                "/videos",
            )),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoSubtitlesFetched(VideoSubtitlesFetched {
                    video_id: video.id.as_str().to_string(),
                    output_dir: "/videos/my-channel".to_string(),
                }),
            )]
        );
    }

    #[test]
    fn it_should_mark_a_video_published_exactly_seven_days_ago_unavailable() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let caption_repository = Arc::new(FakeCaptionRepository::with_probe(Ok(CaptionProbe {
            published_at: Some(now() - Duration::days(7)),
            ..caption_probe(translations_only())
        })));
        let video = Video {
            subtitles: Subtitles::checked(Some(now() - Duration::days(8)), 0),
            ..downloaded_video()
        };
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            video_transcript_repository.clone(),
            caption_repository.clone(),
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository.clone(), "/videos")),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 1,
                updated_at: now(),
                subtitles: Subtitles {
                    filename: video.subtitles.filename.clone(),
                    ..Subtitles::unavailable_at(Some(now()))
                },
                ..video.clone()
            }]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_not_publish_that_subtitles_were_fetched_if_unavailable() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let caption_repository = Arc::new(FakeCaptionRepository::with_probe(Ok(caption_probe(
            translations_only(),
        ))));
        let video = Video {
            subtitles: Subtitles::checked(Some(now() - Duration::days(8)), 0),
            ..downloaded_video()
        };
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            caption_repository,
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(
                Arc::new(FakeVideoFileRepository::default()),
                "/videos",
            )),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(result, Ok(TaskOutcome::Done));
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[test]
    fn it_should_not_publish_that_subtitles_were_fetched_if_the_fetch_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let caption_repository = Arc::new(
            FakeCaptionRepository::with_probe(Ok(caption_probe(english_captions())))
                .with_subtitles(Err(anyhow::anyhow!("HTTP Error 429: Too Many Requests"))),
        );
        let video = downloaded_video();
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
        let task = FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            caption_repository,
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(
                Arc::new(FakeVideoFileRepository::default()),
                "/videos",
            )),
        )));

        let result = run(&task, &task_for(video.id.as_str()));

        assert_eq!(
            result,
            Ok(TaskOutcome::RunAgainAt {
                run_at: now() + Duration::minutes(10),
                reason: "HTTP Error 429: Too Many Requests".to_string(),
            })
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    /// A downloaded channel episode numbered `2026`/`031501`, its subtitles
    /// still missing, created 30 days before `now`.
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

    fn subtitles_task(
        db: &TestDatabase,
        video_repository: Arc<SqliteVideoRepository>,
        caption_repository: Arc<FakeCaptionRepository>,
    ) -> FetchSubtitlesTask {
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        FetchSubtitlesTask::new(Arc::new(SubtitleFetcher::new(
            video_repository,
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            caption_repository,
            video_file_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(FixedClock(now())),
            Arc::new(StagingArea::new(video_file_repository, "/videos")),
        )))
    }

    fn my_channel_id() -> ChannelHandle {
        ChannelHandle::new("@mychannel").unwrap()
    }

    /// The channel holding the video a test fetches subtitles for, its show
    /// folder `/videos/my-channel`.
    fn my_channel() -> Channel {
        Channel::create(
            my_channel_id(),
            "My Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            ReconcileInterval::default_interval(),
            PlaylistPath::new("my-channel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    fn downloaded_video() -> Video {
        Video::create(
            YoutubeVideoId::new("yt1").unwrap(),
            "Title",
            fixed_timestamp(),
        )
        .with_episode_number(
            EpisodeNumber::from_columns(2026, 31501).unwrap(),
            fixed_timestamp(),
        )
        .mark_downloaded(
            Quality::High,
            "Season 2026/S2026E031501 - Title.mp4",
            None,
            None,
            fixed_timestamp(),
        )
    }

    /// An English video with uploaded English captions.
    fn english_captions() -> CaptionTracks {
        CaptionTracks {
            original_language: Some("en".to_string()),
            manual: vec!["en".to_string()],
            automatic: vec!["en-orig".to_string()],
        }
    }

    /// A Spanish video whose only tracks are in other languages.
    fn translations_only() -> CaptionTracks {
        CaptionTracks {
            original_language: Some("es".to_string()),
            manual: vec!["en".to_string()],
            automatic: vec!["en".to_string(), "fr".to_string()],
        }
    }

    /// Published at `fixed_timestamp`, 30 days before `now`.
    fn caption_probe(tracks: CaptionTracks) -> CaptionProbe {
        CaptionProbe {
            tracks,
            published_at: Some(fixed_timestamp()),
            info_json: PathBuf::from("yt1.info.json"),
            warnings: vec![],
        }
    }

    /// Subtitles staged at `path`, relative to the staging dir.
    fn fetched_subtitles(path: &str) -> FetchedSubtitles {
        FetchedSubtitles {
            path: PathBuf::from(path),
            srt: SRT.to_string(),
        }
    }

    const SRT: &str = "1\n00:00:00,000 --> 00:00:02,000\nHello there\n\n2\n00:00:02,000 --> 00:00:04,000\nGeneral Kenobi\n";

    fn staging_dir(video: &Video) -> PathBuf {
        PathBuf::from(format!(
            "/videos/.downloads/{}-subtitles",
            video.id.as_str()
        ))
    }

    fn pending_event(id: i64, event: DomainEvent) -> ScheduledEvent {
        ScheduledEvent {
            id,
            event_type: event.event_type().to_string(),
            payload: event.payload().to_string(),
            retries: 0,
            created_at: now(),
            updated_at: now(),
            last_error: None,
        }
    }

    fn task_for(video_id: &str) -> Task {
        Task::FetchSubtitles {
            video_id: video_id.to_string(),
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-03-15T10:00:00Z")
            .unwrap()
            .to_utc()
    }

    /// When the fetch runs: 30 days after `fixed_timestamp`.
    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-04-14T10:00:00Z")
            .unwrap()
            .to_utc()
    }

    fn run(task: &FetchSubtitlesTask, queued: &Task) -> Result<TaskOutcome, String> {
        task.handle(queued).map_err(|e| e.to_string())
    }
}
