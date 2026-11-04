use super::video_limit::VideoLimit;
use crate::domain::shared::{Quality, ReconcileInterval};

/// Settings to change on a channel; `None` keeps the stored value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChannelSettingsChange {
    pub quality: Option<Quality>,
    pub video_limit: Option<VideoLimit>,
    pub reconcile_interval: Option<ReconcileInterval>,
    pub summaries_enabled: Option<bool>,
}

impl ChannelSettingsChange {
    pub fn is_empty(&self) -> bool {
        self.quality.is_none()
            && self.video_limit.is_none()
            && self.reconcile_interval.is_none()
            && self.summaries_enabled.is_none()
    }
}
