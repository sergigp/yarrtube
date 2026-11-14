use crate::domain::task::Task;

/// One of the Plex syncs the daemon runs on a schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlexSyncKind {
    WatchState,
    Labels,
}

impl PlexSyncKind {
    pub const ALL: [Self; 2] = [Self::WatchState, Self::Labels];

    /// The kind named `value` (`watch_state` or `labels`), `None` for any
    /// other.
    pub fn new(_value: &str) -> Option<Self> {
        None
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WatchState => "watch_state",
            Self::Labels => "labels",
        }
    }

    /// The task that runs this sync.
    pub fn task(&self) -> Task {
        match self {
            Self::WatchState => Task::SyncPlexWatchState,
            Self::Labels => Task::SyncPlexLabels,
        }
    }
}
