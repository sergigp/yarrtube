use crate::domain::services::{PlexFolderScanner, PlexFolderScannerApi};
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct VideoSubtitlesFetchedPayload {
    output_dir: String,
}

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
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoSubtitlesFetchedPayload = serde_json::from_str(payload)?;
        self.scanner.scan_folder(Path::new(&payload.output_dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::plex_library_repository::FakePlexLibraryRepository;
    use std::sync::Arc;

    #[test]
    fn it_should_scan_the_show_folder_after_subtitles_are_fetched() {
        let plex_repository = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/volume1/media/yarrtube/channels")
                .with_location("20", "/volume1/media/yarrtube/playlists"),
        );
        let subscriber = ScanPlexFolderOnVideoSubtitlesFetched::new(PlexFolderScanner::new(
            "/videos",
            "/volume1/media/yarrtube",
            plex_repository.clone(),
        ));

        let result = handle(&subscriber, &payload("/videos/channels/Some Channel"));

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.mutations(),
            vec!["scan:19:/volume1/media/yarrtube/channels/Some Channel".to_string()]
        );
    }

    fn payload(output_dir: &str) -> String {
        serde_json::json!({
            "video_id": "rec1",
            "output_dir": output_dir,
        })
        .to_string()
    }

    fn handle(
        subscriber: &ScanPlexFolderOnVideoSubtitlesFetched,
        payload: &str,
    ) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
