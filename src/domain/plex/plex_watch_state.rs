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
    #[cfg(test)]
    pub fn unplayed() -> Self {
        Self {
            view_count: 0,
            view_offset_ms: None,
            last_viewed_at: None,
            duration_ms: None,
        }
    }

    /// A started offset (see `PlaybackPosition::is_started`, against the
    /// episode's duration) counts as in progress, floored to seconds; else
    /// viewed at least once ⇒ watched; else unplayed.
    pub fn progress(&self) -> WatchProgress {
        match (self.resume_position(), self.view_count) {
            (Some(position), _) => WatchProgress::InProgress(position),
            (None, 0) => WatchProgress::Unplayed,
            (None, _) => WatchProgress::Watched,
        }
    }

    /// The offset in whole seconds, when started against the duration.
    fn resume_position(&self) -> Option<PlaybackPosition> {
        self.view_offset_ms
            .and_then(|offset_ms| PlaybackPosition::new(offset_ms / 1000).ok())
            .filter(|position| position.is_started(self.duration_seconds()))
    }

    fn duration_seconds(&self) -> Option<i64> {
        self.duration_ms.map(|duration_ms| duration_ms / 1000)
    }
}
