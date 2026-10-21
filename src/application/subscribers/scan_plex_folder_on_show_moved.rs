use crate::domain::services::{PlexFolderScanner, PlexFolderScannerApi};
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ShowMovedPayload {
    from: String,
    to: String,
}

/// Reacts to `ShowMoved` by asking Plex to scan the show's new folder and
/// its old one, so Plex picks the show up at its new location and drops it
/// from the old. Registered only when the Plex integration is configured. A
/// Plex failure fails the event, so it is retried.
pub struct ScanPlexFolderOnShowMoved {
    scanner: PlexFolderScanner,
}

impl ScanPlexFolderOnShowMoved {
    pub fn new(scanner: PlexFolderScanner) -> Self {
        Self { scanner }
    }
}

impl EventSubscriber for ScanPlexFolderOnShowMoved {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: ShowMovedPayload = serde_json::from_str(payload)?;
        self.scanner.scan_folder(Path::new(&payload.to))?;
        self.scanner.scan_folder(Path::new(&payload.from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::plex_library_repository::FakePlexLibraryRepository;
    use std::sync::Arc;

    #[test]
    fn it_should_scan_the_new_and_old_folders() {
        let plex_repository = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/volume1/media/yarrtube/music")
                .with_location("20", "/volume1/media/yarrtube/archive"),
        );
        let subscriber = ScanPlexFolderOnShowMoved::new(PlexFolderScanner::new(
            "/videos",
            "/volume1/media/yarrtube",
            plex_repository.clone(),
        ));

        let result = handle(
            &subscriber,
            &payload("/videos/music/chill", "/videos/archive/chill"),
        );

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.mutations(),
            vec![
                "scan:20:/volume1/media/yarrtube/archive/chill".to_string(),
                "scan:19:/volume1/media/yarrtube/music/chill".to_string(),
            ]
        );
    }

    #[test]
    fn it_should_fail_if_plex_is_unreachable() {
        let subscriber = ScanPlexFolderOnShowMoved::new(PlexFolderScanner::new(
            "/videos",
            "/volume1/media/yarrtube",
            Arc::new(FakePlexLibraryRepository::failing()),
        ));

        let result = handle(
            &subscriber,
            &payload("/videos/music/chill", "/videos/archive/chill"),
        );

        assert_eq!(result, Err("Plex is unreachable".to_string()));
    }

    fn payload(from: &str, to: &str) -> String {
        serde_json::json!({ "from": from, "to": to }).to_string()
    }

    fn handle(subscriber: &ScanPlexFolderOnShowMoved, payload: &str) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
