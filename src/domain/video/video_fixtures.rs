//! Test-only builders for `Video` and its value objects, kept out of the
//! production files so a new field doesn't sweep through them.

use super::retry_attempts::MAX_ATTEMPTS;
use super::video::Video;
use super::video_chapter::VideoChapter;
use super::youtube_video_id::YoutubeVideoId;
use crate::domain::shared::Quality;
use chrono::{DateTime, Utc};

/// `youtube_id` (titled `Video <youtube_id>`) downloaded `now` at High
/// quality to `filename`, with no thumbnail or duration recorded.
pub fn downloaded_video(youtube_id: &str, filename: &str, now: DateTime<Utc>) -> Video {
    Video::create(
        YoutubeVideoId::new(youtube_id).unwrap(),
        format!("Video {youtube_id}"),
        now,
    )
    .mark_downloaded(Quality::High, filename, None, None, now)
}

impl Video {
    /// A download that gave up, after every allowed attempt failed `now`
    /// with `reason`.
    pub fn mark_download_failed(self, reason: &str, now: DateTime<Utc>) -> Self {
        (0..MAX_ATTEMPTS).fold(self, |video, _| video.mark_errored(reason, now))
    }

    /// A summary that gave up, after every allowed attempt failed `now`.
    pub fn mark_summary_failed(self, now: DateTime<Utc>) -> Self {
        (0..MAX_ATTEMPTS).fold(self, |video, _| video.record_summary_failure(now))
    }

    /// A categorization that gave up, after every allowed attempt failed
    /// `now`.
    pub fn mark_categorization_failed(self, now: DateTime<Utc>) -> Self {
        (0..MAX_ATTEMPTS).fold(self, |video, _| video.record_categorization_failure(now))
    }

    /// Sets the chapters alone, without recording a check
    /// (`record_chapters_check` is the production transition).
    pub fn with_chapters(self, chapters: Vec<VideoChapter>, now: DateTime<Utc>) -> Self {
        Self {
            chapters: Some(chapters),
            updated_at: now,
            ..self
        }
    }

    /// Subtitles Unavailable without recording a check
    /// (`record_subtitles_check` is the production transition).
    pub fn mark_subtitles_unavailable(self, now: DateTime<Utc>) -> Self {
        Self {
            subtitles: super::LateDataChecks {
                state: self.subtitles.state.unavailable(),
                ..self.subtitles
            },
            updated_at: now,
            ..self
        }
    }
}
