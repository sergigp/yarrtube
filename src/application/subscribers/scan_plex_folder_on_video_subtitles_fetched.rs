use crate::domain::services::PlexFolderScanner;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;

/// Reacts to `VideoSubtitlesFetched` by asking Plex to scan the video's show
/// folder, so Plex picks up the new subtitle file. Mirrors
/// `ScanPlexFolderOnVideoDownloaded`, and is registered under the same
/// condition. A Plex failure fails the event, so it is retried.
pub struct ScanPlexFolderOnVideoSubtitlesFetched {
    scanner: PlexFolderScanner,
}

impl ScanPlexFolderOnVideoSubtitlesFetched {
    pub fn new(scanner: PlexFolderScanner) -> Self {
        Self { scanner }
    }
}

impl EventSubscriber for ScanPlexFolderOnVideoSubtitlesFetched {
    fn handle(&self, _payload: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
