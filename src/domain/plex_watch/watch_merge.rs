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
pub fn merge(local: WatchSide, remote: WatchSide, agreed: WatchProgress) -> MergeOutcome {
    if local.progress == remote.progress {
        MergeOutcome::InSync(local.progress)
    } else if remote.progress == agreed {
        MergeOutcome::Push(local.progress)
    } else if local.progress == agreed || remote.changed_later_than(&local) {
        MergeOutcome::Pull(remote.progress)
    } else {
        MergeOutcome::Push(local.progress)
    }
}

impl WatchSide {
    /// Whether this side changed after `other`; a side with no known time of
    /// change never did.
    fn changed_later_than(&self, other: &WatchSide) -> bool {
        match (self.changed_at, other.changed_at) {
            (Some(mine), Some(theirs)) => mine > theirs,
            (Some(_), None) => true,
            (None, _) => false,
        }
    }
}
