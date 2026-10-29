use crate::domain::services::plex_label_syncer::AFTER_DOWNLOAD_DELAY;
use crate::domain::services::{PlexLabelSyncer, PlexLabelSyncerApi};
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

/// The only field this subscriber reads from `VideoDownloaded`.
#[derive(Debug, Deserialize)]
struct VideoDownloadedPayload {
    video_id: String,
}

/// Reacts to `VideoDownloaded` by marking the record's Plex labels stale
/// and requesting a label sync in a minute, once Plex has imported the
/// episode. Registered only when the Plex integration is configured.
pub struct SyncPlexLabelsOnVideoDownloaded {
    plex_label_syncer: PlexLabelSyncer,
}

impl SyncPlexLabelsOnVideoDownloaded {
    pub fn new(plex_label_syncer: PlexLabelSyncer) -> Self {
        Self { plex_label_syncer }
    }
}

impl EventSubscriber for SyncPlexLabelsOnVideoDownloaded {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoDownloadedPayload = serde_json::from_str(payload)?;
        let video_id = VideoRecordId::new(payload.video_id)?;
        self.plex_label_syncer
            .mark(&[video_id], AFTER_DOWNLOAD_DELAY)
    }
}
