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
    pub fn new(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_parse_watch_state() {
        assert_eq!(
            PlexSyncKind::new("watch_state"),
            Some(PlexSyncKind::WatchState)
        );
    }

    #[test]
    fn it_should_parse_labels() {
        assert_eq!(PlexSyncKind::new("labels"), Some(PlexSyncKind::Labels));
    }

    #[test]
    fn it_should_reject_any_other_value() {
        assert_eq!(PlexSyncKind::new("scan"), None);
    }

    #[test]
    fn it_should_round_trip_through_its_string() {
        assert_eq!(
            PlexSyncKind::new(PlexSyncKind::Labels.as_str()),
            Some(PlexSyncKind::Labels)
        );
    }

    #[test]
    fn it_should_name_the_task_that_runs_it() {
        assert_eq!(PlexSyncKind::WatchState.task(), Task::SyncPlexWatchState);
        assert_eq!(PlexSyncKind::Labels.task(), Task::SyncPlexLabels);
    }
}
