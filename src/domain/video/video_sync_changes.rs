use super::VideoSyncStep;

/// What one video sync did — see the `video-sync` capability: the steps it
/// found due (each scheduled unless already queued or running), and the
/// repairs it made. Empty when the video needed nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VideoSyncChanges {
    pub steps_due: Vec<VideoSyncStep>,
    pub reset_for_redownload: bool,
    pub metadata_repaired: bool,
    pub relocated: bool,
}

impl VideoSyncChanges {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}
