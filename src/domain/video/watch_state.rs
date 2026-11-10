use super::playback_position::PlaybackPosition;
use chrono::{DateTime, Duration, Utc};

/// How long after it was last played a started video is still offered to
/// continue watching.
const IN_PROGRESS_WINDOW: Duration = Duration::days(7);

/// How far past the start playback must have got for a video to count as
/// started rather than just opened.
const IN_PROGRESS_MIN_POSITION_SECONDS: i64 = 30;

const WATCHED_THRESHOLD: f64 = 0.9;
const REWATCH_RESET_THRESHOLD: f64 = 0.1;

/// Whether a video was watched, how far playback got and when it was last
/// played.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchState {
    pub watched_at: Option<DateTime<Utc>>,
    pub position: PlaybackPosition,
    pub last_played_at: Option<DateTime<Utc>>,
}

impl WatchState {
    /// Never watched nor played.
    pub fn unwatched() -> Self {
        Self {
            watched_at: None,
            position: PlaybackPosition::start(),
            last_played_at: None,
        }
    }

    pub fn is_watched(&self) -> bool {
        self.watched_at.is_some()
    }

    /// Unwatched, position over 30s and last played within the past 7 days
    /// of `now`: worth offering to continue watching.
    pub fn is_in_progress(&self, now: DateTime<Utc>) -> bool {
        !self.is_watched()
            && self.position.seconds() > IN_PROGRESS_MIN_POSITION_SECONDS
            && self
                .last_played_at
                .is_some_and(|played_at| now - played_at <= IN_PROGRESS_WINDOW)
    }

    /// Watched `now`, its position back to the start.
    pub fn mark_watched(self, now: DateTime<Utc>) -> Self {
        Self {
            watched_at: Some(now),
            position: PlaybackPosition::start(),
            ..self
        }
    }

    /// Updates the watch state from how far playback got against
    /// `duration_seconds`, the known duration. An unwatched video becomes
    /// watched at 90%; a watched one becomes unwatched again once a rewatch
    /// passes 10%, as long as it is still below 90%. With no known duration
    /// an unwatched video only keeps the position. Whatever the outcome, the
    /// video counts as played `now`. A report is stale when its playback
    /// began with the video unwatched (`was_watched` false) but the video is
    /// now watched: it was marked watched since, so the state is returned
    /// unchanged, not even played.
    pub fn record_playback(
        self,
        position: PlaybackPosition,
        duration_seconds: Option<i64>,
        was_watched: bool,
        now: DateTime<Utc>,
    ) -> Self {
        if !was_watched && self.is_watched() {
            return self;
        }
        let progress = duration_seconds.map(|duration| position.seconds() as f64 / duration as f64);
        let updated = match (self.is_watched(), progress) {
            (true, Some(progress))
                if progress > REWATCH_RESET_THRESHOLD && progress < WATCHED_THRESHOLD =>
            {
                Self {
                    watched_at: None,
                    position,
                    ..self
                }
            }
            (true, _) => self,
            (false, Some(progress)) if progress >= WATCHED_THRESHOLD => self.mark_watched(now),
            (false, _) => Self { position, ..self },
        };
        Self {
            last_played_at: Some(now),
            ..updated
        }
    }
}
