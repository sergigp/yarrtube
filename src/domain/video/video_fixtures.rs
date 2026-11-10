//! Test-only builders for `Video` and its value objects, kept out of the
//! production files so a new field doesn't sweep through them.

use super::chapters::Chapters;
use super::downloaded_quality::DownloadedQuality;
use super::retry_attempts::{MAX_ATTEMPTS, RetryAttempts};
use super::step_state::StepState;
use super::step_status::StepStatus;
use super::subtitles::Subtitles;
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
            chapters: Chapters {
                found: Some(chapters),
                ..self.chapters
            },
            updated_at: now,
            ..self
        }
    }

    /// Subtitles Unavailable without recording a check
    /// (`record_subtitles_check` is the production transition).
    pub fn mark_subtitles_unavailable(self, now: DateTime<Utc>) -> Self {
        Self {
            subtitles: Subtitles {
                state: self.subtitles.state.unavailable(),
                ..self.subtitles
            },
            updated_at: now,
            ..self
        }
    }
}

impl Subtitles {
    /// Subtitles last checked at `last_checked_at` with `failed` consecutive
    /// failed checks — Pending without failures, else Retrying (Failed from
    /// `MAX_ATTEMPTS`), the last failed at the last check.
    pub fn checked(last_checked_at: Option<DateTime<Utc>>, failed: u32) -> Self {
        Self {
            state: checked_state(last_checked_at, failed),
            last_checked_at,
            filename: None,
        }
    }

    /// Subtitles found (Done) by the check at `last_checked_at`, no file
    /// recorded.
    pub fn done_at(last_checked_at: Option<DateTime<Utc>>) -> Self {
        Self {
            state: StepState::pending().succeed(),
            last_checked_at,
            filename: None,
        }
    }

    /// Subtitles found missing on an old video (Unavailable) by the check at
    /// `last_checked_at`.
    pub fn unavailable_at(last_checked_at: Option<DateTime<Utc>>) -> Self {
        Self {
            state: StepState::pending().unavailable(),
            last_checked_at,
            filename: None,
        }
    }
}

impl Chapters {
    /// Chapters last checked at `last_checked_at` with `failed` consecutive
    /// failed checks, none found.
    pub fn checked(last_checked_at: Option<DateTime<Utc>>, failed: u32) -> Self {
        Self {
            state: checked_state(last_checked_at, failed),
            last_checked_at,
            found: None,
        }
    }

    /// Chapters found (Done) by the check at `last_checked_at`, none
    /// recorded.
    pub fn done_at(last_checked_at: Option<DateTime<Utc>>) -> Self {
        Self {
            state: StepState::pending().succeed(),
            last_checked_at,
            found: None,
        }
    }

    /// Chapters found missing on an old video (Unavailable) by the check at
    /// `last_checked_at`.
    pub fn unavailable_at(last_checked_at: Option<DateTime<Utc>>) -> Self {
        Self {
            state: StepState::pending().unavailable(),
            last_checked_at,
            found: None,
        }
    }
}

impl DownloadedQuality {
    /// A quality last checked at `last_checked_at` with `failed` consecutive
    /// failed checks, no height recorded.
    pub fn checked(last_checked_at: Option<DateTime<Utc>>, failed: u32) -> Self {
        Self {
            state: checked_state(last_checked_at, failed),
            last_checked_at,
            height: None,
        }
    }

    /// A quality settled (Done) by the check at `last_checked_at`, no
    /// height recorded.
    pub fn done_at(last_checked_at: Option<DateTime<Utc>>) -> Self {
        Self {
            state: StepState::pending().succeed(),
            last_checked_at,
            height: None,
        }
    }
}

/// A late-data step last checked at `last_checked_at` with `failed`
/// consecutive failed checks.
fn checked_state(last_checked_at: Option<DateTime<Utc>>, failed: u32) -> StepState {
    let status = match failed {
        0 => StepStatus::Pending,
        f if f >= MAX_ATTEMPTS => StepStatus::Failed,
        _ => StepStatus::Retrying,
    };
    StepState {
        status,
        attempts: RetryAttempts {
            failed,
            last_failed_at: last_checked_at.filter(|_| failed > 0),
        },
    }
}
