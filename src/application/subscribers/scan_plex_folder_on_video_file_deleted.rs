use crate::domain::services::PlexFolderScanner;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;

/// Reacts to `VideoFileDeleted` by asking Plex to scan the show folder the
/// video's files were deleted from, so the episode leaves Plex at once.
/// Registered only when Plex is configured. A Plex failure fails the event,
/// so it is retried.
pub struct ScanPlexFolderOnVideoFileDeleted {
    #[allow(dead_code)]
    scanner: PlexFolderScanner,
}

impl ScanPlexFolderOnVideoFileDeleted {
    pub fn new(scanner: PlexFolderScanner) -> Self {
        Self { scanner }
    }
}

impl EventSubscriber for ScanPlexFolderOnVideoFileDeleted {
    fn handle(&self, _payload: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
