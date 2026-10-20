use crate::domain::shared::ReconcileInterval;

/// Settings to change on a playlist; `None` keeps the stored value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlaylistSettingsChange {
    pub exclude_from_home: Option<bool>,
    pub hide_unwatched_count: Option<bool>,
    pub reconcile_interval: Option<ReconcileInterval>,
}

impl PlaylistSettingsChange {
    pub fn is_empty(&self) -> bool {
        self.exclude_from_home.is_none()
            && self.hide_unwatched_count.is_none()
            && self.reconcile_interval.is_none()
    }
}
