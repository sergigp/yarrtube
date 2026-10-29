use crate::domain::services::plex_label_syncer::CATEGORY_CHANGE_DELAY;
use crate::domain::services::{PlexLabelSyncer, PlexLabelSyncerApi};
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct VideoCategoriesChangedPayload {
    video_ids: Vec<String>,
}

/// Reacts to `VideoCategoriesChanged` by marking the records' Plex labels
/// stale and requesting a label sync shortly. Registered only when the Plex
/// integration is configured.
pub struct SyncPlexLabelsOnVideoCategoriesChanged {
    plex_label_syncer: PlexLabelSyncer,
}

impl SyncPlexLabelsOnVideoCategoriesChanged {
    pub fn new(plex_label_syncer: PlexLabelSyncer) -> Self {
        Self { plex_label_syncer }
    }
}

impl EventSubscriber for SyncPlexLabelsOnVideoCategoriesChanged {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoCategoriesChangedPayload = serde_json::from_str(payload)?;
        let video_ids = payload
            .video_ids
            .into_iter()
            .map(VideoRecordId::new)
            .collect::<Result<Vec<_>, _>>()?;
        self.plex_label_syncer
            .mark(&video_ids, CATEGORY_CHANGE_DELAY)
    }
}
