use crate::domain::video::WatchProgress;
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
        if self.view_count > 0 {
            WatchProgress::Watched
        } else {
            WatchProgress::Unplayed
        }
    }
}
