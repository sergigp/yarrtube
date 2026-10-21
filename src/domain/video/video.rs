use super::episode_number::EpisodeNumber;
use super::playback_position::PlaybackPosition;
use super::subtitle_state::SubtitleState;
use super::video_duration::VideoDuration;
use super::video_id::VideoId;
use super::video_output_entry::is_episode_path;
use super::video_record_id::VideoRecordId;
use super::video_status::VideoStatus;
use crate::domain::shared::Quality;
use chrono::{DateTime, Duration, Utc};

/// How long a video stays permanently errored before reconcile gives it
/// another download attempt.
const ERRORED_RECOVERY_COOLDOWN: Duration = Duration::hours(24);

/// How long after it was last played a started video is still offered to
/// continue watching.
const IN_PROGRESS_WINDOW: Duration = Duration::days(7);

/// How far past the start playback must have got for a video to count as
/// started rather than just opened.
const IN_PROGRESS_MIN_POSITION_SECONDS: i64 = 30;

/// A video shorter than this (15 minutes) counts as a quick watch.
const QUICK_WATCH_MAX_DURATION_SECONDS: i64 = 900;

/// A downloaded (or to-be-downloaded) video's own record: its download
/// state, independent of any container. Container membership — which
/// playlist(s) or channel(s) this video belongs to, and its position within
/// each — is recorded separately by `PlaylistVideo`/`ChannelVideo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    pub id: VideoRecordId,
    pub youtube_id: VideoId,
    pub title: String,
    pub status: VideoStatus,
    pub quality: Option<Quality>,
    pub filename: Option<String>,
    pub thumbnail_filename: Option<String>,
    pub duration_seconds: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub watched_at: Option<DateTime<Utc>>,
    pub playback_position: PlaybackPosition,
    pub synced_at: Option<DateTime<Utc>>,
    pub last_errored_at: Option<DateTime<Utc>>,
    pub last_played_at: Option<DateTime<Utc>>,
    /// The video's episode number within its show, assigned once by the
    /// first thumbnail fetch, download or relocation that places a file.
    pub episode_number: Option<EpisodeNumber>,
    pub subtitle_state: SubtitleState,
    /// The episode's subtitle file, recorded once its subtitles are fetched.
    pub subtitle_filename: Option<String>,
    /// When a conclusive subtitle fetch first found no selectable track:
    /// starts the 7-day window before the subtitles are given up on.
    pub subtitles_first_missing_at: Option<DateTime<Utc>>,
}

const WATCHED_THRESHOLD: f64 = 0.9;
const REWATCH_RESET_THRESHOLD: f64 = 0.1;

impl Video {
    pub fn create(youtube_id: VideoId, title: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            id: VideoRecordId::new_generated(),
            youtube_id,
            title: title.into(),
            status: VideoStatus::Pending,
            quality: None,
            filename: None,
            thumbnail_filename: None,
            duration_seconds: None,
            created_at: now,
            updated_at: now,
            watched_at: None,
            playback_position: PlaybackPosition::start(),
            synced_at: None,
            last_errored_at: None,
            last_played_at: None,
            episode_number: None,
            subtitle_state: SubtitleState::Missing,
            subtitle_filename: None,
            subtitles_first_missing_at: None,
        }
    }

    pub fn start_download(self, now: DateTime<Utc>) -> Self {
        Self {
            status: VideoStatus::InProgress,
            updated_at: now,
            ..self
        }
    }

    pub fn mark_downloaded(
        self,
        quality: Quality,
        filename: impl Into<String>,
        thumbnail_filename: Option<String>,
        duration_seconds: Option<i64>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            status: VideoStatus::Downloaded,
            quality: Some(quality),
            filename: Some(filename.into()),
            thumbnail_filename,
            duration_seconds,
            updated_at: now,
            synced_at: Some(now),
            ..self
        }
    }

    pub fn mark_errored_retrying(self, now: DateTime<Utc>) -> Self {
        Self {
            status: VideoStatus::ErroredRetrying,
            updated_at: now,
            ..self
        }
    }

    pub fn mark_errored(self, now: DateTime<Utc>) -> Self {
        Self {
            status: VideoStatus::Errored,
            updated_at: now,
            last_errored_at: Some(now),
            ..self
        }
    }

    /// Marks a video excluded: a terminal status for a video that can never
    /// be downloaded (see the `video-download` capability). Deliberately
    /// leaves `last_errored_at` untouched — recovery keys off `Errored`, and
    /// `Excluded` is a distinct status outside that predicate, so an excluded
    /// video is never reset by reconcile recovery.
    pub fn mark_excluded(self, now: DateTime<Utc>) -> Self {
        Self {
            status: VideoStatus::Excluded,
            updated_at: now,
            ..self
        }
    }

    /// Records a thumbnail fetched independently of, and ahead of, the
    /// video's full download — see the `video-thumbnails` capability.
    /// Touches only `thumbnail_filename`/`updated_at`, leaving `status`,
    /// `filename`, and `quality` exactly as they were. Stored through
    /// `VideoRepository::update_thumbnail`, which writes only that column.
    #[cfg(test)]
    pub fn with_thumbnail(self, thumbnail_filename: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            thumbnail_filename: Some(thumbnail_filename.into()),
            updated_at: now,
            ..self
        }
    }

    /// Resets a video back to `Pending`, clearing its recorded filename,
    /// thumbnail filename, quality and subtitles (back to `Missing` with no
    /// first check without captions, so the redownload refetches them),
    /// used by filesystem reconciliation both when a `Downloaded` video's
    /// recorded file is missing from disk and when an `Errored` video is
    /// given another chance, in each case followed by scheduling a fresh
    /// download.
    pub fn reset_for_redownload(self, now: DateTime<Utc>) -> Self {
        Self {
            status: VideoStatus::Pending,
            quality: None,
            filename: None,
            thumbnail_filename: None,
            duration_seconds: None,
            updated_at: now,
            synced_at: None,
            subtitle_state: SubtitleState::Missing,
            subtitle_filename: None,
            subtitles_first_missing_at: None,
            ..self
        }
    }

    /// Updates the watch state from how far playback got, against the
    /// recorded duration or, when none is recorded, the one the player
    /// reported. An unwatched video becomes watched at 90%; a watched one
    /// becomes unwatched again once a rewatch passes 10%, as long as it is
    /// still below 90%. With no known duration an unwatched video only keeps
    /// the position. Whatever the outcome, the video counts as played `now`.
    /// A report is stale when its playback began with the video unwatched
    /// (`was_watched` false) but the video is now watched: it was marked
    /// watched since, so the video is returned unchanged, not even played.
    pub fn update_watch_state(
        self,
        position: PlaybackPosition,
        reported_duration: Option<VideoDuration>,
        was_watched: bool,
        now: DateTime<Utc>,
    ) -> Self {
        if !was_watched && self.is_watched() {
            return self;
        }
        let progress = self
            .known_duration_seconds(reported_duration)
            .map(|duration| position.seconds() as f64 / duration as f64);
        let updated = match (self.is_watched(), progress) {
            (true, Some(progress))
                if progress > REWATCH_RESET_THRESHOLD && progress < WATCHED_THRESHOLD =>
            {
                Self {
                    watched_at: None,
                    playback_position: position,
                    ..self
                }
            }
            (true, _) => self,
            (false, Some(progress)) if progress >= WATCHED_THRESHOLD => self.mark_watched(now),
            (false, _) => Self {
                playback_position: position,
                ..self
            },
        };
        Self {
            last_played_at: Some(now),
            ..updated
        }
    }

    pub fn mark_watched(self, now: DateTime<Utc>) -> Self {
        Self {
            watched_at: Some(now),
            playback_position: PlaybackPosition::start(),
            ..self
        }
    }

    /// This video with `current`'s watch state: `watched_at`,
    /// `playback_position` and `last_played_at`.
    pub fn with_watch_state_of(self, current: &Video) -> Self {
        Self {
            watched_at: current.watched_at,
            playback_position: current.playback_position,
            last_played_at: current.last_played_at,
            ..self
        }
    }

    pub fn is_watched(&self) -> bool {
        self.watched_at.is_some()
    }

    pub fn is_downloaded(&self) -> bool {
        self.status == VideoStatus::Downloaded
    }

    /// Downloaded and not yet watched: counted in unwatched badges.
    pub fn is_downloaded_and_unwatched(&self) -> bool {
        self.is_downloaded() && !self.is_watched()
    }

    /// Downloaded, unwatched, position over 30s and last played within the
    /// past 7 days of `now`: worth offering to continue watching.
    pub fn is_in_progress(&self, now: DateTime<Utc>) -> bool {
        !self.is_watched()
            && self.playback_position.seconds() > IN_PROGRESS_MIN_POSITION_SECONDS
            && self
                .last_played_at
                .is_some_and(|played_at| now - played_at <= IN_PROGRESS_WINDOW)
    }

    /// Downloaded, unwatched and recorded as shorter than 15 minutes.
    pub fn is_quick_watch(&self) -> bool {
        !self.is_watched()
            && self
                .duration_seconds
                .is_some_and(|duration| duration < QUICK_WATCH_MAX_DURATION_SECONDS)
    }

    /// Whether reconcile should reset this video for another download: it
    /// has been permanently errored for at least the recovery cooldown,
    /// counted from when it last errored. An errored video with no recorded
    /// time (errored before that time was tracked) is due straight away.
    pub fn is_due_for_recovery(&self, now: DateTime<Utc>) -> bool {
        self.status == VideoStatus::Errored
            && self
                .last_errored_at
                .is_none_or(|errored_at| now - errored_at >= ERRORED_RECOVERY_COOLDOWN)
    }

    /// Whether the video's download has settled (Downloaded, Excluded or
    /// Errored), so a queued download has nothing left to do for it and a
    /// video in any other status with no download queued is stranded.
    pub fn is_download_settled(&self) -> bool {
        matches!(
            self.status,
            VideoStatus::Downloaded | VideoStatus::Excluded | VideoStatus::Errored
        )
    }

    /// Whether a thumbnail fetch can still be worth attempting for this
    /// video: never for an `Excluded` one, which is permanently unavailable,
    /// nor an `Errored` one, whose recovery redownload writes its own.
    pub fn is_thumbnail_fetchable(&self) -> bool {
        !matches!(self.status, VideoStatus::Excluded | VideoStatus::Errored)
    }

    /// Whether a subtitle fetch is worth attempting: Downloaded, numbered,
    /// in its season folder (a legacy-layout file has no episode path to
    /// place the subtitles next to) and its subtitles still Missing.
    pub fn is_subtitle_fetchable(&self) -> bool {
        self.is_downloaded()
            && self.episode_number.is_some()
            && !self.is_in_legacy_layout()
            && self.subtitle_state == SubtitleState::Missing
    }

    /// Records the fetched subtitle file: subtitle state Fetched. Touches
    /// only the subtitle fields and `updated_at`, stored through
    /// `VideoRepository::update_subtitles` only.
    pub fn with_subtitles(self, subtitle_filename: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            subtitle_state: SubtitleState::Fetched,
            subtitle_filename: Some(subtitle_filename.into()),
            updated_at: now,
            ..self
        }
    }

    /// The language code the fetched subtitle file is named with (`en` for
    /// `… .en.srt`); `None` unless the subtitles are Fetched.
    pub fn subtitle_language(&self) -> Option<String> {
        self.subtitle_filename
            .as_deref()
            .filter(|_| self.subtitle_state == SubtitleState::Fetched)
            .and_then(|filename| filename.strip_suffix(".srt"))
            .and_then(|stem| stem.rsplit_once('.'))
            .map(|(_, language)| language.to_string())
    }

    /// The video has no original-language captions: a terminal subtitle
    /// state.
    pub fn mark_subtitles_unavailable(self, now: DateTime<Utc>) -> Self {
        Self {
            subtitle_state: SubtitleState::Unavailable,
            updated_at: now,
            ..self
        }
    }

    /// Records `now` as the first check without captions, unless one is
    /// already recorded.
    pub fn mark_subtitles_first_missing(self, now: DateTime<Utc>) -> Self {
        match self.subtitles_first_missing_at {
            Some(_) => self,
            None => Self {
                subtitles_first_missing_at: Some(now),
                updated_at: now,
                ..self
            },
        }
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
        Self {
            filename: Some(filename.into()),
            thumbnail_filename,
            episode_number: Some(number),
            updated_at: now,
            ..self
        }
    }

    /// Whether the video's recorded file is in the per-video-folder (or flat)
    /// layout that predates the TV-show layout, i.e. not under a `Season `
    /// folder.
    pub fn is_in_legacy_layout(&self) -> bool {
        self.filename
            .as_deref()
            .is_some_and(|filename| !is_episode_path(filename))
    }

    /// The recorded duration, else the reported one. The reported one is
    /// always positive (`VideoDuration`), but yt-dlp truncates a sub-second
    /// video's duration to a recorded 0, which counts as unknown so progress
    /// is never divided by it.
    fn known_duration_seconds(&self, reported_duration: Option<VideoDuration>) -> Option<i64> {
        self.duration_seconds
            .or(reported_duration.map(|duration| duration.seconds()))
            .filter(|duration| *duration > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_build_a_pending_video() {
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", now);

        assert_eq!(video.status, VideoStatus::Pending);
        assert_eq!(video.title, "My Video");
        assert_eq!(video.quality, None);
        assert_eq!(video.filename, None);
        assert_eq!(video.thumbnail_filename, None);
        assert_eq!(video.duration_seconds, None);
        assert_eq!(video.created_at, now);
        assert_eq!(video.updated_at, now);
    }

    #[test]
    fn it_should_generate_a_distinct_surrogate_id_per_video() {
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

        let a = Video::create(VideoId::new("vid1").unwrap(), "First", now);
        let b = Video::create(VideoId::new("vid2").unwrap(), "Second", now);

        assert_ne!(a.id, b.id);
    }

    fn video() -> Video {
        Video::create(
            VideoId::new("vid1").unwrap(),
            "My Video",
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
    }

    #[test]
    fn it_should_transition_to_in_progress_when_a_download_starts() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().start_download(now);

        assert_eq!(video.status, VideoStatus::InProgress);
        assert_eq!(video.quality, None);
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

        assert_eq!(video.status, VideoStatus::Downloaded);
        assert_eq!(video.quality, Some(Quality::High));
        assert_eq!(video.filename, Some("My Video.mp4".to_string()));
        assert_eq!(video.thumbnail_filename, Some("My Video.jpg".to_string()));
        assert_eq!(video.duration_seconds, Some(223));
        assert_eq!(video.updated_at, now);
    }

    #[test]
    fn it_should_record_no_thumbnail_filename_when_marked_downloaded_without_one() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_downloaded(Quality::High, "My Video.mp4", None, None, now);

        assert_eq!(video.thumbnail_filename, None);
    }

    #[test]
    fn it_should_record_no_duration_when_marked_downloaded_without_one() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_downloaded(Quality::High, "My Video.mp4", None, None, now);

        assert_eq!(video.duration_seconds, None);
    }

    #[test]
    fn it_should_only_touch_thumbnail_filename_and_updated_at_when_recording_a_thumbnail() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().with_thumbnail("My Video/My Video.jpg", now);

        assert_eq!(
            video.thumbnail_filename,
            Some("My Video/My Video.jpg".to_string())
        );
        assert_eq!(video.updated_at, now);
        assert_eq!(video.status, VideoStatus::Pending);
        assert_eq!(video.filename, None);
        assert_eq!(video.quality, None);
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

        assert_eq!(reset.status, VideoStatus::Pending);
        assert_eq!(reset.quality, None);
        assert_eq!(reset.filename, None);
        assert_eq!(reset.thumbnail_filename, None);
        assert_eq!(reset.duration_seconds, None);
        assert_eq!(reset.updated_at, later);
    }

    #[test]
    fn it_should_transition_to_errored_retrying_when_marked_errored_retrying() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_errored_retrying(now);

        assert_eq!(video.status, VideoStatus::ErroredRetrying);
        assert_eq!(video.quality, None);
        assert_eq!(video.updated_at, now);
    }

    #[test]
    fn it_should_transition_to_errored_when_marked_errored() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_errored(now);

        assert_eq!(video.status, VideoStatus::Errored);
        assert_eq!(video.quality, None);
        assert_eq!(video.updated_at, now);
        assert_eq!(video.last_errored_at, Some(now));
    }

    #[test]
    fn it_should_transition_to_excluded_when_marked_excluded() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let video = video();

        let excluded = video.clone().mark_excluded(now);

        assert_eq!(
            excluded,
            Video {
                status: VideoStatus::Excluded,
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
        let video = video();

        let relocated = video.clone().relocate_to_episode(
            number,
            "Season 1/S01E003 - My Video.mp4",
            Some("Season 1/S01E003 - My Video.jpg".to_string()),
            now,
        );

        assert_eq!(
            relocated,
            Video {
                filename: Some("Season 1/S01E003 - My Video.mp4".to_string()),
                thumbnail_filename: Some("Season 1/S01E003 - My Video.jpg".to_string()),
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
            .with_subtitles("Season 1/S01E003 - My Video.en.srt", now);

        assert_eq!(
            fetched,
            Video {
                subtitle_state: SubtitleState::Fetched,
                subtitle_filename: Some("Season 1/S01E003 - My Video.en.srt".to_string()),
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
                subtitle_state: SubtitleState::Unavailable,
                updated_at: now,
                ..video
            }
        );
    }

    #[test]
    fn it_should_report_the_subtitle_language_from_its_filename() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let filename = "Season 1/S01E003 - My Video.en.srt";

        let languages = [
            video().with_subtitles(filename, now),
            video(),
            Video {
                subtitle_state: SubtitleState::Unavailable,
                subtitle_filename: Some(filename.to_string()),
                ..video()
            },
        ]
        .map(|video| video.subtitle_language());

        assert_eq!(languages, [Some("en".to_string()), None, None]);
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
            .with_subtitles("Season 1/S01E003 - My Video.en.srt", now);

        let reset = fetched.clone().reset_for_redownload(later);

        assert_eq!(
            reset,
            Video {
                status: VideoStatus::Pending,
                quality: None,
                filename: None,
                subtitle_state: SubtitleState::Missing,
                subtitle_filename: None,
                updated_at: later,
                synced_at: None,
                ..fetched
            }
        );
    }

    #[test]
    fn it_should_clear_the_first_check_without_captions_when_reset_for_redownload() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let later = DateTime::<Utc>::from_timestamp(200, 0).unwrap();
        let checked = video()
            .mark_downloaded(
                Quality::High,
                "Season 1/S01E003 - My Video.mp4",
                None,
                None,
                now,
            )
            .mark_subtitles_first_missing(now);

        let reset = checked.clone().reset_for_redownload(later);

        assert_eq!(
            reset,
            Video {
                status: VideoStatus::Pending,
                quality: None,
                filename: None,
                subtitles_first_missing_at: None,
                updated_at: later,
                synced_at: None,
                ..checked
            }
        );
    }

    #[test]
    fn it_should_keep_when_it_last_errored_when_reset_for_redownload() {
        let errored_at = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let later = DateTime::<Utc>::from_timestamp(200, 0).unwrap();

        let reset = video().mark_errored(errored_at).reset_for_redownload(later);

        assert_eq!(reset.last_errored_at, Some(errored_at));
    }

    #[test]
    fn it_should_be_due_for_recovery_once_errored_for_the_whole_cooldown() {
        let errored_at = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_errored(errored_at);

        assert!(video.is_due_for_recovery(errored_at + Duration::hours(24)));
    }

    #[test]
    fn it_should_be_due_for_recovery_when_errored_longer_than_the_cooldown() {
        let errored_at = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_errored(errored_at);

        assert!(video.is_due_for_recovery(errored_at + Duration::days(3)));
    }

    #[test]
    fn it_should_measure_the_recovery_cooldown_from_when_it_errored() {
        let errored_at = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let refreshed_at = errored_at + Duration::hours(23);

        let video = Video {
            updated_at: refreshed_at,
            ..video().mark_errored(errored_at)
        };

        assert!(video.is_due_for_recovery(errored_at + Duration::hours(24)));
    }

    #[test]
    fn it_should_be_due_for_recovery_if_errored_with_no_recorded_time() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = Video {
            status: VideoStatus::Errored,
            last_errored_at: None,
            ..video()
        };

        assert!(video.is_due_for_recovery(now));
    }

    #[test]
    fn it_should_not_be_due_for_recovery_if_errored_within_the_cooldown() {
        let errored_at = DateTime::<Utc>::from_timestamp(100, 0).unwrap();

        let video = video().mark_errored(errored_at);

        assert!(
            !video.is_due_for_recovery(errored_at + Duration::hours(24) - Duration::seconds(1))
        );
    }

    #[test]
    fn it_should_never_be_due_for_recovery_if_excluded() {
        let errored_at = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let long_after = errored_at + Duration::days(3650);

        let excluded = Video {
            status: VideoStatus::Excluded,
            updated_at: errored_at,
            last_errored_at: Some(errored_at),
            ..video()
        };

        assert!(!excluded.is_due_for_recovery(long_after));
    }

    #[test]
    fn it_should_not_be_due_for_recovery_if_not_permanently_errored() {
        let updated_at = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let later = updated_at + Duration::days(3);

        let pending = video();
        let retrying = video().mark_errored_retrying(updated_at);
        let in_progress = video().start_download(updated_at);

        assert_eq!(
            [pending, retrying, in_progress].map(|v| v.is_due_for_recovery(later)),
            [false, false, false]
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
    fn it_should_be_settled_only_when_downloaded_errored_or_excluded() {
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let videos = [
            video(),
            video().start_download(now),
            video().mark_downloaded(Quality::High, "My Video.mp4", None, None, now),
            video().mark_errored_retrying(now),
            video().mark_errored(now),
            video().mark_excluded(now),
        ];

        let settled: Vec<bool> = videos.iter().map(Video::is_download_settled).collect();

        assert_eq!(settled, vec![false, false, true, false, true, true]);
    }
}
