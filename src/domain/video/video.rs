use super::chapters::Chapters;
use super::download::Download;
use super::downloaded_file::DownloadedFile;
use super::downloaded_quality::DownloadedQuality;
use super::episode_number::EpisodeNumber;
use super::late_data_outcome::LateDataOutcome;
use super::playback_position::PlaybackPosition;
use super::step_state::StepState;
use super::step_status::StepStatus;
use super::subtitles::Subtitles;
use super::thumbnail::Thumbnail;
use super::video_chapter::VideoChapter;
use super::video_duration::VideoDuration;
use super::video_id::VideoId;
use super::video_status::VideoStatus;
use super::watch_state::WatchState;
use super::youtube_video_id::YoutubeVideoId;
use crate::domain::shared::Quality;
use chrono::{DateTime, Utc};

/// A video shorter than this (15 minutes) counts as a quick watch.
const QUICK_WATCH_MAX_DURATION_SECONDS: i64 = 900;

/// A stored (downloaded or to-be-downloaded) video, identified by its own
/// `id` (one per source holding it), not its `youtube_id`: its download
/// state, independent of any container. Container membership — which
/// playlist(s) or channel(s) this video belongs to, and its position within
/// each — is recorded separately by `PlaylistVideo`/`ChannelVideo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    pub id: VideoId,
    pub youtube_id: YoutubeVideoId,
    pub title: String,
    pub download: Download,
    pub thumbnail: Thumbnail,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
    /// The video's episode number within its show, assigned once by the
    /// first thumbnail fetch, download or relocation that places a file.
    pub episode_number: Option<EpisodeNumber>,
    pub subtitles: Subtitles,
    pub chapters: Chapters,
    pub downloaded_quality: DownloadedQuality,
    pub summary: StepState,
    /// The AI categorization's step state — see the `video-categorization`
    /// capability.
    pub categorization: StepState,
    pub watch: WatchState,
    /// Bumped by every stored update; an update only applies on top of the
    /// version it was read at — see the `video-write-consistency` capability.
    pub version: u32,
}

impl Video {
    pub fn create(
        youtube_id: YoutubeVideoId,
        title: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: VideoId::new_generated(),
            youtube_id,
            title: title.into(),
            download: Download::pending(),
            thumbnail: Thumbnail::pending(),
            created_at: now,
            updated_at: now,
            synced_at: None,
            episode_number: None,
            subtitles: Subtitles::pending(),
            chapters: Chapters::pending(),
            downloaded_quality: DownloadedQuality::pending(),
            summary: StepState::pending(),
            categorization: StepState::pending(),
            watch: WatchState::unwatched(),
            version: 0,
        }
    }

    pub fn start_download(self, now: DateTime<Utc>) -> Self {
        Self {
            download: self.download.start(),
            updated_at: now,
            ..self
        }
    }

    /// Records the download done, its failed attempts and failure cleared;
    /// a recorded thumbnail makes the thumbnail Done. A download without a
    /// thumbnail keeps the one fetched ahead of it.
    pub fn mark_downloaded(
        self,
        quality: Quality,
        filename: impl Into<String>,
        thumbnail_filename: Option<String>,
        duration_seconds: Option<i64>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            download: self.download.succeed(DownloadedFile {
                filename: filename.into(),
                quality,
                duration_seconds,
            }),
            thumbnail: self.thumbnail.record_download(thumbnail_filename),
            updated_at: now,
            synced_at: Some(now),
            ..self
        }
    }

    /// Records a failed download attempt and why: Retrying, or Failed on the
    /// last allowed attempt (see the `video-retries` capability).
    pub fn mark_errored(self, reason: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            download: self.download.fail(reason, now),
            updated_at: now,
            ..self
        }
    }

    /// Marks a video's download Unavailable: final, for a video that can
    /// never be downloaded (see the `video-download` capability), its failed
    /// attempts and failure cleared.
    pub fn mark_unavailable(self, now: DateTime<Utc>) -> Self {
        Self {
            download: self.download.unavailable(),
            updated_at: now,
            ..self
        }
    }

    /// Every Retrying or Failed step back to Pending with no failed
    /// attempts — see the `video-retries` capability. Pending, In Progress,
    /// Done and Unavailable steps are left as they are.
    pub fn retry(self, now: DateTime<Utc>) -> Self {
        Self {
            download: self.download.retry(),
            thumbnail: self.thumbnail.retry(),
            subtitles: self.subtitles.retry(),
            chapters: self.chapters.retry(),
            downloaded_quality: self.downloaded_quality.retry(),
            summary: self.summary.retry(),
            categorization: self.categorization.retry(),
            updated_at: now,
            ..self
        }
    }

    /// This video retitled, as its source now lists it.
    pub fn with_title(self, title: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            title: title.into(),
            updated_at: now,
            ..self
        }
    }

    /// This video as stored after one more update.
    pub fn next_version(self) -> Self {
        Self {
            version: self.version + 1,
            ..self
        }
    }

    /// Records a thumbnail fetched independently of, and ahead of, the
    /// video's full download — see the `video-thumbnails` capability.
    /// Touches only the thumbnail and `updated_at`, leaving the download
    /// exactly as it was.
    pub fn with_thumbnail(self, thumbnail_filename: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            thumbnail: self.thumbnail.succeed(thumbnail_filename),
            updated_at: now,
            ..self
        }
    }

    /// Records a thumbnail fetch that obtained no thumbnail.
    pub fn record_thumbnail_failure(self, now: DateTime<Utc>) -> Self {
        Self {
            thumbnail: self.thumbnail.fail(now),
            updated_at: now,
            ..self
        }
    }

    /// Whether a thumbnail fetch should run at `now`.
    pub fn is_thumbnail_due(&self, now: DateTime<Utc>) -> bool {
        self.thumbnail.filename.is_none()
            && self.is_thumbnail_fetchable()
            && !self.download.started
            && self.thumbnail.state.is_due(now)
    }

    /// Resets a video back to `Pending`, clearing its recorded filename,
    /// thumbnail, quality and subtitles, and clearing the late-data checks
    /// and downloaded height so the redownload refetches them, used by
    /// filesystem reconciliation when a `Downloaded` video's recorded file is
    /// missing from disk, followed by scheduling a fresh download.
    pub fn reset_for_redownload(self, now: DateTime<Utc>) -> Self {
        Self {
            download: Download::pending(),
            thumbnail: Thumbnail::pending(),
            updated_at: now,
            synced_at: None,
            subtitles: Subtitles::pending(),
            chapters: self.chapters.reset_checks(),
            downloaded_quality: DownloadedQuality::pending(),
            ..self
        }
    }

    /// Updates the watch state from how far playback got, against the
    /// recorded duration or, when none is recorded, the one the player
    /// reported — see `WatchState::record_playback`.
    pub fn update_watch_state(
        self,
        position: PlaybackPosition,
        reported_duration: Option<VideoDuration>,
        was_watched: bool,
        now: DateTime<Utc>,
    ) -> Self {
        let duration_seconds = self.known_duration_seconds(reported_duration);
        Self {
            watch: self
                .watch
                .record_playback(position, duration_seconds, was_watched, now),
            ..self
        }
    }

    pub fn mark_watched(self, now: DateTime<Utc>) -> Self {
        Self {
            watch: self.watch.mark_watched(now),
            ..self
        }
    }

    /// Downloaded and not yet watched: counted in unwatched badges.
    pub fn is_downloaded_and_unwatched(&self) -> bool {
        self.download.is_downloaded() && !self.watch.is_watched()
    }

    /// Downloaded, unwatched and recorded as shorter than 15 minutes.
    pub fn is_quick_watch(&self) -> bool {
        !self.watch.is_watched()
            && self
                .download
                .duration_seconds()
                .is_some_and(|duration| duration < QUICK_WATCH_MAX_DURATION_SECONDS)
    }

    /// Whether a thumbnail fetch can still be worth attempting for this
    /// video: never for an Unavailable download, nor a Retrying or Failed
    /// one, whose next download writes its own.
    pub fn is_thumbnail_fetchable(&self) -> bool {
        !matches!(
            self.download.status(),
            VideoStatus::Unavailable | VideoStatus::Retrying | VideoStatus::Failed
        )
    }

    /// Whether a subtitle fetch is worth attempting: Downloaded, numbered,
    /// in its season folder (a legacy-layout file has no episode path to
    /// place the subtitles next to) and its subtitles Pending or Retrying.
    pub fn is_subtitle_fetchable(&self) -> bool {
        self.download.is_downloaded()
            && self.episode_number.is_some()
            && !self.download.is_in_legacy_layout()
            && self.subtitles.is_unsettled()
    }

    /// Records the fetched subtitle file: subtitles Done, checked `now`.
    /// Touches only the subtitle fields and `updated_at`.
    pub fn mark_subtitles_fetched(
        self,
        subtitle_filename: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            subtitles: self.subtitles.fetched(subtitle_filename, now),
            updated_at: now,
            ..self
        }
    }

    /// Whether a chapters fetch is worth attempting: Downloaded and its
    /// chapters Pending or Retrying.
    pub fn is_chapters_fetchable(&self) -> bool {
        self.download.is_downloaded() && self.chapters.is_unsettled()
    }

    /// Downloaded, subtitles Done, summary Pending or Retrying.
    pub fn is_summarizable(&self) -> bool {
        self.download.is_downloaded()
            && self.subtitles.state.status == StepStatus::Done
            && matches!(
                self.summary.status,
                StepStatus::Pending | StepStatus::Retrying
            )
    }

    /// Whether a categorization should run at `now`: Pending or Done (a
    /// category may be pending since), or Retrying with its next attempt due.
    pub fn is_categorization_due(&self, now: DateTime<Utc>) -> bool {
        self.categorization.status == StepStatus::Done || self.categorization.is_due(now)
    }

    /// Summary Done, its failed attempts cleared.
    pub fn mark_summary_generated(self, now: DateTime<Utc>) -> Self {
        Self {
            summary: self.summary.succeed(),
            updated_at: now,
            ..self
        }
    }

    /// Records a failed summary attempt: Retrying, or Failed on the last.
    pub fn record_summary_failure(self, now: DateTime<Utc>) -> Self {
        Self {
            summary: self.summary.fail(now),
            updated_at: now,
            ..self
        }
    }

    /// Records a categorization run that left nothing rejected: Done.
    pub fn mark_categorized(self, now: DateTime<Utc>) -> Self {
        Self {
            categorization: self.categorization.succeed(),
            updated_at: now,
            ..self
        }
    }

    /// Records a failed categorization attempt: Retrying, or Failed on the
    /// last.
    pub fn record_categorization_failure(self, now: DateTime<Utc>) -> Self {
        Self {
            categorization: self.categorization.fail(now),
            updated_at: now,
            ..self
        }
    }

    /// Categorization back to Pending (a category was added or its verdicts
    /// removed), its failed attempts cleared.
    pub fn reset_categorization(self, now: DateTime<Utc>) -> Self {
        Self {
            categorization: StepState::pending(),
            updated_at: now,
            ..self
        }
    }

    /// Records a subtitle check that found no subtitles (`Incomplete`) or
    /// failed, under the `late-video-data` policy. Touches only the subtitle
    /// fields and `updated_at`.
    pub fn record_subtitles_check(
        self,
        outcome: LateDataOutcome,
        published_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            subtitles: self.subtitles.record_check(outcome, published_at, now),
            updated_at: now,
            ..self
        }
    }

    /// Records a chapters check: `Some` non-empty chapters are `Complete`,
    /// `Some(vec![])` `Incomplete`, `None` a failed check (keeping the
    /// chapters already recorded, if any). Touches only the chapters fields
    /// and `updated_at`.
    pub fn record_chapters_check(
        self,
        chapters: Option<Vec<VideoChapter>>,
        published_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            chapters: self.chapters.record_check(chapters, published_at, now),
            updated_at: now,
            ..self
        }
    }

    /// Records the tier and height a download or quality upgrade landed at,
    /// making the quality Done when the video is old or the tier's target
    /// height is reached; a `degraded` one (SABR reported) counts as a failed
    /// check instead. Touches only the quality fields and `updated_at`.
    pub fn record_downloaded_quality(
        self,
        quality: Quality,
        height: Option<u32>,
        degraded: bool,
        published_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Self {
        let file = self
            .download
            .file
            .clone()
            .map(|file| file.with_quality(quality));
        Self {
            download: self.download.with_file(file),
            downloaded_quality: self.downloaded_quality.record_download(
                quality,
                height,
                degraded,
                published_at,
                now,
            ),
            updated_at: now,
            ..self
        }
    }

    /// Records a quality check that found nothing better (`probed_height`
    /// the height the tier would get now) or failed (`None`). Touches only
    /// the quality fields and `updated_at`.
    pub fn record_quality_check(
        self,
        probed_height: Option<u32>,
        published_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            downloaded_quality: self.downloaded_quality.record_check(
                probed_height,
                published_at,
                now,
            ),
            updated_at: now,
            ..self
        }
    }

    /// Whether a quality check is worth attempting: Downloaded and its
    /// quality Pending or Retrying.
    pub fn is_quality_checkable(&self) -> bool {
        self.download.is_downloaded() && self.downloaded_quality.is_unsettled()
    }

    /// Records the episode number assigned to this video.
    pub fn with_episode_number(self, number: EpisodeNumber, now: DateTime<Utc>) -> Self {
        Self {
            episode_number: Some(number),
            updated_at: now,
            ..self
        }
    }

    /// Records a legacy video's files at their new paths in its season
    /// folder, under the episode number they were relocated for.
    pub fn relocate_to_episode(
        self,
        number: EpisodeNumber,
        filename: impl Into<String>,
        thumbnail_filename: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        let file = self
            .download
            .file
            .clone()
            .map(|file| file.with_filename(filename));
        Self {
            download: self.download.with_file(file),
            thumbnail: self.thumbnail.with_filename(thumbnail_filename),
            episode_number: Some(number),
            updated_at: now,
            ..self
        }
    }

    /// The recorded duration, else the reported one. The reported one is
    /// always positive (`VideoDuration`), but yt-dlp truncates a sub-second
    /// video's duration to a recorded 0, which counts as unknown so progress
    /// is never divided by it.
    fn known_duration_seconds(&self, reported_duration: Option<VideoDuration>) -> Option<i64> {
        self.download
            .duration_seconds()
            .or(reported_duration.map(|duration| duration.seconds()))
            .filter(|duration| *duration > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::video::RetryAttempts;

    #[test]
    fn it_should_build_a_pending_video() {
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let video = Video::create(YoutubeVideoId::new("vid1").unwrap(), "My Video", now);

        assert_eq!(video.download, Download::pending());
        assert_eq!(video.title, "My Video");
        assert_eq!(video.thumbnail.filename, None);
        assert_eq!(video.created_at, now);
        assert_eq!(video.updated_at, now);
    }

    #[test]
    fn it_should_generate_a_distinct_surrogate_id_per_video() {
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let a = Video::create(YoutubeVideoId::new("vid1").unwrap(), "First", now);
        let b = Video::create(YoutubeVideoId::new("vid2").unwrap(), "Second", now);

        assert_ne!(a.id, b.id);
    }

    fn video() -> Video {
        Video::create(
            YoutubeVideoId::new("vid1").unwrap(),
            "My Video",
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
    }

    #[test]
    fn it_should_transition_to_in_progress_when_a_download_starts() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().start_download(now);

        assert_eq!(video.download.status(), VideoStatus::InProgress);
        assert_eq!(video.download.file, None);
        assert_eq!(video.updated_at, now);
    }

    #[test]
    fn it_should_transition_to_downloaded_when_marked_downloaded() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_downloaded(
            Quality::High,
            "My Video.mp4",
            Some("My Video.jpg".to_string()),
            Some(223),
            now,
        );

        assert_eq!(video.download.status(), VideoStatus::Downloaded);
        assert_eq!(
            video.download.file,
            Some(DownloadedFile {
                filename: "My Video.mp4".to_string(),
                quality: Quality::High,
                duration_seconds: Some(223),
            })
        );
        assert_eq!(video.thumbnail.filename, Some("My Video.jpg".to_string()));
        assert_eq!(video.updated_at, now);
    }

    #[test]
    fn it_should_record_no_thumbnail_filename_when_marked_downloaded_without_one() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_downloaded(Quality::High, "My Video.mp4", None, None, now);

        assert_eq!(video.thumbnail.filename, None);
    }

    #[test]
    fn it_should_record_no_duration_when_marked_downloaded_without_one() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_downloaded(Quality::High, "My Video.mp4", None, None, now);

        assert_eq!(video.download.duration_seconds(), None);
    }

    #[test]
    fn it_should_only_touch_thumbnail_filename_and_updated_at_when_recording_a_thumbnail() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().with_thumbnail("My Video/My Video.jpg", now);

        assert_eq!(
            video.thumbnail.filename,
            Some("My Video/My Video.jpg".to_string())
        );
        assert_eq!(video.updated_at, now);
        assert_eq!(video.download, Download::pending());
    }

    #[test]
    fn it_should_reset_a_downloaded_video_back_to_pending_for_redownload() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let downloaded = video().mark_downloaded(
            Quality::High,
            "My Video.mp4",
            Some("My Video.jpg".to_string()),
            Some(223),
            now,
        );

        let later = DateTime::<Utc>::from_timestamp(200, 0).unwrap();
        let reset = downloaded.reset_for_redownload(later);

        assert_eq!(reset.download, Download::pending());
        assert_eq!(reset.thumbnail.filename, None);
        assert_eq!(reset.updated_at, later);
    }

    #[test]
    fn it_should_transition_to_retrying_when_marked_errored() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let video = video();

        let errored = video.clone().mark_errored("HTTP Error 503", now);

        assert_eq!(
            errored,
            Video {
                download: Download {
                    state: StepState {
                        status: StepStatus::Retrying,
                        attempts: RetryAttempts {
                            failed: 1,
                            last_failed_at: Some(now),
                        },
                    },
                    started: false,
                    failure: Some("HTTP Error 503".to_string()),
                    file: None,
                },
                updated_at: now,
                ..video
            }
        );
    }

    #[test]
    fn it_should_transition_to_unavailable_when_marked_unavailable() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let video = video();

        let unavailable = video.clone().mark_unavailable(now);

        assert_eq!(
            unavailable,
            Video {
                download: Download {
                    state: StepState::pending().unavailable(),
                    ..Download::pending()
                },
                updated_at: now,
                ..video
            }
        );
    }

    #[test]
    fn it_should_record_its_episode_number() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let number = EpisodeNumber::from_columns(1, 3).unwrap();
        let video = video();

        let numbered = video.clone().with_episode_number(number, now);

        assert_eq!(
            numbered,
            Video {
                episode_number: Some(number),
                updated_at: now,
                ..video
            }
        );
    }

    #[test]
    fn it_should_record_its_files_when_relocated_to_an_episode() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let number = EpisodeNumber::from_columns(1, 3).unwrap();
        let video = video().mark_downloaded(
            Quality::High,
            "My Video/My Video.mp4",
            Some("My Video/My Video.jpg".to_string()),
            Some(223),
            now,
        );

        let relocated = video.clone().relocate_to_episode(
            number,
            "Season 1/S01E003 - My Video.mp4",
            Some("Season 1/S01E003 - My Video.jpg".to_string()),
            now,
        );

        assert_eq!(
            relocated,
            Video {
                download: Download {
                    file: Some(DownloadedFile {
                        filename: "Season 1/S01E003 - My Video.mp4".to_string(),
                        quality: Quality::High,
                        duration_seconds: Some(223),
                    }),
                    ..video.download.clone()
                },
                thumbnail: Thumbnail {
                    filename: Some("Season 1/S01E003 - My Video.jpg".to_string()),
                    ..video.thumbnail.clone()
                },
                episode_number: Some(number),
                updated_at: now,
                ..video
            }
        );
    }

    #[test]
    fn it_should_record_its_fetched_subtitles() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let video = video();

        let fetched = video
            .clone()
            .mark_subtitles_fetched("Season 1/S01E003 - My Video.en.srt", now);

        assert_eq!(
            fetched,
            Video {
                subtitles: Subtitles {
                    state: StepState::pending().succeed(),
                    last_checked_at: Some(now),
                    filename: Some("Season 1/S01E003 - My Video.en.srt".to_string()),
                },
                updated_at: now,
                ..video
            }
        );
    }

    #[test]
    fn it_should_mark_its_subtitles_unavailable() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let video = video();

        let unavailable = video.clone().mark_subtitles_unavailable(now);

        assert_eq!(
            unavailable,
            Video {
                subtitles: Subtitles {
                    state: StepState::pending().unavailable(),
                    last_checked_at: None,
                    filename: None,
                },
                updated_at: now,
                ..video
            }
        );
    }

    #[test]
    fn it_should_record_its_generated_summary() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let video = video();

        let generated = video.clone().mark_summary_generated(now);

        assert_eq!(
            generated,
            Video {
                summary: StepState::pending().succeed(),
                updated_at: now,
                ..video
            }
        );
    }

    #[test]
    fn it_should_clear_its_subtitles_when_reset_for_redownload() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let later = DateTime::<Utc>::from_timestamp(200, 0).unwrap();
        let fetched = video()
            .mark_downloaded(
                Quality::High,
                "Season 1/S01E003 - My Video.mp4",
                None,
                None,
                now,
            )
            .mark_subtitles_fetched("Season 1/S01E003 - My Video.en.srt", now);

        let reset = fetched.clone().reset_for_redownload(later);

        assert_eq!(
            reset,
            Video {
                download: Download::pending(),
                subtitles: Subtitles::pending(),
                updated_at: later,
                synced_at: None,
                ..fetched
            }
        );
    }

    #[test]
    fn it_should_clear_the_subtitle_checks_when_reset_for_redownload() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let later = DateTime::<Utc>::from_timestamp(200, 0).unwrap();
        let checked = Video {
            subtitles: Subtitles {
                state: StepState::pending().fail(now).fail(now),
                last_checked_at: Some(now),
                filename: None,
            },
            ..video().mark_downloaded(
                Quality::High,
                "Season 1/S01E003 - My Video.mp4",
                None,
                None,
                now,
            )
        };

        let reset = checked.clone().reset_for_redownload(later);

        assert_eq!(
            reset,
            Video {
                download: Download::pending(),
                subtitles: Subtitles::pending(),
                updated_at: later,
                synced_at: None,
                ..checked
            }
        );
    }

    #[test]
    fn it_should_not_be_subtitle_fetchable_if_in_the_legacy_layout() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let number = EpisodeNumber::from_columns(1, 3).unwrap();
        let downloaded_at = |filename: &str| {
            video().with_episode_number(number, now).mark_downloaded(
                Quality::High,
                filename,
                None,
                None,
                now,
            )
        };

        let fetchable = [
            "Season 1/S01E003 - My Video.mp4",
            "My Video/My Video.mp4",
            "My Video.mp4",
        ]
        .map(|filename| downloaded_at(filename).is_subtitle_fetchable());

        assert_eq!(fetchable, [true, false, false]);
    }

    #[test]
    fn it_should_record_the_time_it_was_retitled() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let video = video();

        let retitled = video.clone().with_title("Renamed", now);

        assert_eq!(
            retitled,
            Video {
                title: "Renamed".to_string(),
                updated_at: now,
                ..video
            }
        );
    }
}
