use super::playback_position::PlaybackPosition;
use super::video_duration::VideoDuration;
use super::video_id::VideoId;
use super::video_record_id::VideoRecordId;
use super::video_status::VideoStatus;
use crate::domain::shared::Quality;
use chrono::{DateTime, Utc};

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
            ..self
        }
    }

    /// Records a thumbnail fetched independently of, and ahead of, the
    /// video's full download — see the `video-thumbnails` capability.
    /// Touches only `thumbnail_filename`/`updated_at`, leaving `status`,
    /// `filename`, and `quality` exactly as they were.
    pub fn with_thumbnail(self, thumbnail_filename: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            thumbnail_filename: Some(thumbnail_filename.into()),
            updated_at: now,
            ..self
        }
    }

    /// Resets a video back to `Pending`, clearing its recorded filename,
    /// thumbnail filename, and quality, used by filesystem reconciliation
    /// both when a `Downloaded` video's recorded file is missing from disk
    /// and when an `Errored` video is given another chance, in each case
    /// followed by scheduling a fresh download.
    pub fn reset_for_redownload(self, now: DateTime<Utc>) -> Self {
        Self {
            status: VideoStatus::Pending,
            quality: None,
            filename: None,
            thumbnail_filename: None,
            duration_seconds: None,
            updated_at: now,
            ..self
        }
    }

    /// Updates the watch state from how far playback got, against the
    /// recorded duration or, when none is recorded, the one the player
    /// reported. An unwatched video becomes watched at 90%; a watched one
    /// becomes unwatched again once a rewatch passes 10%, as long as it is
    /// still below 90%. With no known duration an unwatched video only keeps
    /// the position.
    pub fn update_watch_state(
        self,
        position: PlaybackPosition,
        reported_duration: Option<VideoDuration>,
        now: DateTime<Utc>,
    ) -> Self {
        let progress = self
            .known_duration_seconds(reported_duration)
            .map(|duration| position.seconds() as f64 / duration as f64);
        match (self.is_watched(), progress) {
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
        }
    }

    pub fn mark_watched(self, now: DateTime<Utc>) -> Self {
        Self {
            watched_at: Some(now),
            playback_position: PlaybackPosition::start(),
            ..self
        }
    }

    pub fn is_watched(&self) -> bool {
        self.watched_at.is_some()
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
    }
}
