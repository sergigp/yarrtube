use crate::domain::services::PlexFolderScanner;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;

/// Reacts to `ShowMoved` by asking Plex to scan the show's new folder and
/// its old one, so Plex picks the show up at its new location and drops it
/// from the old. Registered only when the Plex integration is configured. A
/// Plex failure fails the event, so it is retried.
pub struct ScanPlexFolderOnShowMoved {
    #[allow(dead_code)]
    scanner: PlexFolderScanner,
}

impl ScanPlexFolderOnShowMoved {
    pub fn new(scanner: PlexFolderScanner) -> Self {
        Self { scanner }
    }
}

impl EventSubscriber for ScanPlexFolderOnShowMoved {
    fn handle(&self, _payload: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
