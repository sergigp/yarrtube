use crate::domain::services::{PlexFolderScanner, PlexFolderScannerApi};
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;
use std::path::Path;

/// The only field this subscriber reads from `VideoFileDeleted`.
#[derive(Debug, Deserialize)]
struct VideoFileDeletedPayload {
    output_dir: String,
}

/// Reacts to `VideoFileDeleted` by asking Plex to scan the show folder the
/// video's files were deleted from, so the episode leaves Plex at once.
/// Registered only when Plex is configured. A Plex failure fails the event,
/// so it is retried.
pub struct ScanPlexFolderOnVideoFileDeleted {
    scanner: PlexFolderScanner,
}

impl ScanPlexFolderOnVideoFileDeleted {
    pub fn new(scanner: PlexFolderScanner) -> Self {
        Self { scanner }
    }
}

impl EventSubscriber for ScanPlexFolderOnVideoFileDeleted {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoFileDeletedPayload = serde_json::from_str(payload)?;
        self.scanner.scan_folder(Path::new(&payload.output_dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::plex_library_repository::FakePlexLibraryRepository;
    use std::sync::Arc;

    #[test]
    fn it_should_scan_the_show_folder_after_a_deletion() {
        let plex_repository = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/volume1/media/yarrtube/channels")
                .with_location("20", "/volume1/media/yarrtube/playlists"),
        );
        let subscriber = ScanPlexFolderOnVideoFileDeleted::new(PlexFolderScanner::new(
            "/videos",
            "/volume1/media/yarrtube",
            plex_repository.clone(),
        ));

        let result = subscriber
            .handle(r#"{"output_dir": "/videos/channels/Some Channel"}"#)
            .map_err(|e| e.to_string());

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.mutations(),
            vec!["scan:19:/volume1/media/yarrtube/channels/Some Channel".to_string()]
        );
    }
}
