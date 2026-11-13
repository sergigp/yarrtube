use crate::domain::video::{PlaybackPosition, WatchProgress};
use chrono::{DateTime, Utc};

/// The token user's watch state of an episode, as Plex lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlexWatchState {
    pub view_count: u32,
    pub view_offset_ms: Option<i64>,
    pub last_viewed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
}

impl PlexWatchState {
    /// Never played.
    pub fn unplayed() -> Self {
        Self {
            view_count: 0,
            view_offset_ms: None,
            last_viewed_at: None,
            duration_ms: None,
        }
    }

    /// An offset counts (in progress, floored to seconds) unless the episode
    /// is watched and the offset is at or below 10% of its duration (a
    /// rewatch just started); else viewed at least once ⇒ watched; else
    /// unplayed.
    pub fn progress(&self) -> WatchProgress {
        match (self.resume_position(), self.view_count) {
            (Some(position), _) => WatchProgress::InProgress(position),
            (None, 0) => WatchProgress::Unplayed,
            (None, _) => WatchProgress::Watched,
        }
    }

    /// The offset in whole seconds, when past the start.
    fn resume_position(&self) -> Option<PlaybackPosition> {
        self.view_offset_ms
            .map(|offset_ms| offset_ms / 1000)
            .filter(|seconds| *seconds > 0)
            .and_then(|seconds| PlaybackPosition::new(seconds).ok())
    }
}
