use crate::domain::video::WatchProgress;
use chrono::{DateTime, Utc};

/// One side's state and when it last changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchSide {
    pub progress: WatchProgress,
    pub changed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeOutcome {
    /// Both sides already show it: record it as agreed.
    InSync(WatchProgress),
    /// Copy Plex's state to yarrtube.
    Pull(WatchProgress),
    /// Copy yarrtube's state to Plex.
    Push(WatchProgress),
}

/// Equal ⇒ in sync; only Plex differs from `agreed` ⇒ pull; only yarrtube
/// ⇒ push; both ⇒ the later `changed_at` wins, a side without one loses,
/// yarrtube on a tie.
pub fn merge(local: WatchSide, _remote: WatchSide, _agreed: WatchProgress) -> MergeOutcome {
    MergeOutcome::InSync(local.progress)
}
