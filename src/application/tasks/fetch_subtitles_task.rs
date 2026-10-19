use crate::domain::services::{SubtitleFetcher, SubtitleFetcherApi};
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::task_handler::TaskHandler;
use std::path::Path;
use std::sync::Arc;

/// Fetches one downloaded video's subtitles, scheduled by the reconcilers'
/// missing-subtitles recovery. Best-effort: always succeeds, so the queue
/// never retries it; the next reconcile pass reschedules it instead.
pub struct FetchSubtitlesTask {
    video_repository: Arc<dyn VideoRepository>,
    subtitle_fetcher: Arc<SubtitleFetcher>,
}

impl FetchSubtitlesTask {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        subtitle_fetcher: Arc<SubtitleFetcher>,
    ) -> Self {
        Self {
            video_repository,
            subtitle_fetcher,
        }
    }
}

impl TaskHandler for FetchSubtitlesTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        let (video_id, output_dir) = Task::decode_fetch_subtitles_payload(payload)?;
        let video_id = VideoRecordId::new(video_id)?;
        let Some(video) = self.video_repository.find(&video_id)? else {
            return Ok(());
        };
        self.subtitle_fetcher.fetch(&video, Path::new(&output_dir));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::shared::Quality;
    use crate::domain::video::{EpisodeNumber, Video, VideoId};
    use crate::domain::video_transcript::{
        CaptionLanguage, CaptionSource, CaptionTracks, TranscriptText, VideoTranscript,
    };
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::repositories::sqlite_video_transcript_repository::{
        SqliteVideoTranscriptRepository, VideoTranscriptRepository,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::{
        CaptionProbe, FakeVideoDownloaderRepository, FetchedSubtitles,
    };
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
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
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_caption_probe(Ok(caption_probe(CaptionTracks {
                    original_language: Some("en".to_string()),
                    manual: vec!["en".to_string(), "fr".to_string()],
                    automatic: vec!["en-orig".to_string(), "en".to_string()],
                })))
                .with_subtitles_result(Ok(fetched_subtitles("yt1.en.srt"))),
        );
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchSubtitlesTask::new(
            video_repository.clone(),
            Arc::new(SubtitleFetcher::new(
                video_repository.clone(),
                video_transcript_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(FixedClock(now())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .clone()
                    .with_subtitles("Season 2026/S2026E031501 - Title.en.srt", now())
            ]
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
    fn it_should_record_auto_generated_captions_as_automatic() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_caption_probe(Ok(caption_probe(CaptionTracks {
                    original_language: Some("es".to_string()),
                    manual: vec!["en".to_string()],
                    automatic: vec!["es-orig".to_string(), "en".to_string()],
                })))
                .with_subtitles_result(Ok(fetched_subtitles("yt1.es-orig.srt"))),
        );
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchSubtitlesTask::new(
            video_repository.clone(),
            Arc::new(SubtitleFetcher::new(
                video_repository.clone(),
                video_transcript_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(FixedClock(now())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                video
                    .clone()
                    .with_subtitles("Season 2026/S2026E031501 - Title.es.srt", now())
            ]
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
    fn it_should_mark_an_old_video_without_captions_unavailable() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_caption_probe(Ok(caption_probe(translations_only()))),
        );
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchSubtitlesTask::new(
            video_repository.clone(),
            Arc::new(SubtitleFetcher::new(
                video_repository.clone(),
                video_transcript_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(FixedClock(now())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().mark_subtitles_unavailable(now())]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_keep_a_recent_video_without_captions_missing() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default().with_caption_probe(Ok(CaptionProbe {
                published_at: Some(now() - Duration::days(2)),
                ..caption_probe(translations_only())
            })),
        );
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchSubtitlesTask::new(
            video_repository.clone(),
            Arc::new(SubtitleFetcher::new(
                video_repository.clone(),
                video_transcript_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(FixedClock(now())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
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
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default().with_caption_probe(Ok(CaptionProbe {
                published_at: None,
                ..caption_probe(translations_only())
            })),
        );
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchSubtitlesTask::new(
            video_repository.clone(),
            Arc::new(SubtitleFetcher::new(
                video_repository.clone(),
                video_transcript_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(FixedClock(now())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().mark_subtitles_unavailable(now())]
        );
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_leave_subtitles_missing_when_the_probe_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_caption_probe(Err(anyhow::anyhow!("HTTP Error 429: Too Many Requests"))),
        );
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchSubtitlesTask::new(
            video_repository.clone(),
            Arc::new(SubtitleFetcher::new(
                video_repository.clone(),
                video_transcript_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(FixedClock(now())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_leave_subtitles_missing_when_the_download_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_caption_probe(Ok(caption_probe(english_captions())))
                .with_subtitles_result(Err(anyhow::anyhow!("HTTP Error 429: Too Many Requests"))),
        );
        let video = downloaded_video();
        video_repository.save(&video).unwrap();
        let task = FetchSubtitlesTask::new(
            video_repository.clone(),
            Arc::new(SubtitleFetcher::new(
                video_repository.clone(),
                video_transcript_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(FixedClock(now())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![video.clone()]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_skip_if_video_no_longer_exists() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(FakeVideoDownloaderRepository::default());

        let task = FetchSubtitlesTask::new(
            video_repository.clone(),
            Arc::new(SubtitleFetcher::new(
                video_repository.clone(),
                video_transcript_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(FixedClock(now())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for("missing"));

        assert_eq!(result, Ok(()));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            *video_downloader_repository.caption_calls.lock().unwrap(),
            0
        );
    }

    #[test]
    fn it_should_skip_if_subtitles_already_fetched() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_caption_probe(Ok(caption_probe(english_captions())))
                .with_subtitles_result(Ok(fetched_subtitles("yt1.en.srt"))),
        );
        let fetched = downloaded_video()
            .with_subtitles("Season 2026/S2026E031501 - Title.en.srt", fixed_timestamp());
        let unavailable = downloaded_video().mark_subtitles_unavailable(fixed_timestamp());
        video_repository.save(&fetched).unwrap();
        video_repository.save(&unavailable).unwrap();
        let task = FetchSubtitlesTask::new(
            video_repository.clone(),
            Arc::new(SubtitleFetcher::new(
                video_repository.clone(),
                video_transcript_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(FixedClock(now())),
                "/videos",
            )),
        );

        let results = [
            run(&task, &payload_for(fetched.id.as_str())),
            run(&task, &payload_for(unavailable.id.as_str())),
        ];

        assert_eq!(results, [Ok(()), Ok(())]);
        assert_eq!(video_repository.list().unwrap(), vec![fetched, unavailable]);
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(
            *video_downloader_repository.caption_calls.lock().unwrap(),
            0
        );
    }

    #[test]
    fn it_should_skip_a_video_that_is_not_downloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_caption_probe(Ok(caption_probe(english_captions())))
                .with_subtitles_result(Ok(fetched_subtitles("yt1.en.srt"))),
        );
        let pending = Video::create(VideoId::new("yt1").unwrap(), "Title", fixed_timestamp())
            .with_episode_number(
                EpisodeNumber::from_columns(2026, 31501).unwrap(),
                fixed_timestamp(),
            );
        let legacy = Video::create(VideoId::new("yt2").unwrap(), "Old", fixed_timestamp())
            .mark_downloaded(Quality::High, "Old/Old.mp4", None, None, fixed_timestamp());
        video_repository.save(&pending).unwrap();
        video_repository.save(&legacy).unwrap();
        let task = FetchSubtitlesTask::new(
            video_repository.clone(),
            Arc::new(SubtitleFetcher::new(
                video_repository.clone(),
                video_transcript_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(FixedClock(now())),
                "/videos",
            )),
        );

        let results = [
            run(&task, &payload_for(pending.id.as_str())),
            run(&task, &payload_for(legacy.id.as_str())),
        ];

        assert_eq!(results, [Ok(()), Ok(())]);
        assert_eq!(video_repository.list().unwrap(), vec![pending, legacy]);
        assert_eq!(
            *video_downloader_repository.caption_calls.lock().unwrap(),
            0
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
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_caption_probe(Ok(caption_probe(english_captions())))
                .with_subtitles_result(Ok(fetched_subtitles("yt1.en.srt"))),
        );
        video_repository.save(&video).unwrap();
        let task = FetchSubtitlesTask::new(
            video_repository.clone(),
            Arc::new(SubtitleFetcher::new(
                video_repository.clone(),
                video_transcript_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(FixedClock(now())),
                "/videos",
            )),
        );

        let result = run(&task, &payload_for(video.id.as_str()));

        assert_eq!(result, Ok(()));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![redownloaded.with_subtitles("Season 2026/S2026E031501 - Title.en.srt", now())]
        );
    }

    #[test]
    fn it_should_discard_the_staging_dir() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::default());
        let video_downloader_repository = Arc::new(
            FakeVideoDownloaderRepository::default()
                .with_caption_probe(Ok(caption_probe(english_captions())))
                .with_subtitles_result(Ok(fetched_subtitles("yt1.en.srt"))),
        );
        let fetched = downloaded_video();
        let failed = downloaded_video();
        video_repository.save(&fetched).unwrap();
        video_repository.save(&failed).unwrap();
        let task = FetchSubtitlesTask::new(
            video_repository.clone(),
            Arc::new(SubtitleFetcher::new(
                video_repository.clone(),
                video_transcript_repository.clone(),
                video_downloader_repository.clone(),
                video_file_repository.clone(),
                Arc::new(SqliteTaskRepository::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(SqliteEventPublisher::new(
                    db.database(),
                    Arc::new(FixedClock(now())),
                )),
                Arc::new(FixedClock(now())),
                "/videos",
            )),
        );

        // The fake's probe is used up by the first fetch: the second one fails.
        let results = [
            run(&task, &payload_for(fetched.id.as_str())),
            run(&task, &payload_for(failed.id.as_str())),
        ];

        assert_eq!(results, [Ok(()), Ok(())]);
        assert_eq!(
            *video_file_repository.deleted_dirs.lock().unwrap(),
            vec![staging_dir(&fetched), staging_dir(&failed)]
        );
    }

    /// A downloaded channel episode numbered `2026`/`031501`, its subtitles
    /// still missing, created 30 days before `now`.
    fn downloaded_video() -> Video {
        Video::create(VideoId::new("yt1").unwrap(), "Title", fixed_timestamp())
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

    fn payload_for(video_id: &str) -> String {
        Task::FetchSubtitles {
            video_id: video_id.to_string(),
            output_dir: "/videos/my-channel".to_string(),
        }
        .payload()
        .to_string()
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

    fn run(task: &FetchSubtitlesTask, payload: &str) -> Result<(), String> {
        task.handle(payload, false).map_err(|e| e.to_string())
    }
}
