use crate::domain::services::{PlexFolderScanner, PlexFolderScannerApi};
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct VideoDownloadedPayload {
    output_dir: String,
    folder: String,
}

/// Reacts to `VideoDownloaded` by asking Plex to scan the video's folder.
/// Registered only when `YARRTUBE_PLEX_URL`, `YARRTUBE_PLEX_TOKEN` and
/// `YARRTUBE_PLEX_VIDEOS_PATH` are configured. A Plex failure fails the event, so it is retried.
pub struct ScanPlexFolderOnVideoDownloaded {
    scanner: PlexFolderScanner,
}

impl ScanPlexFolderOnVideoDownloaded {
    pub fn new(scanner: PlexFolderScanner) -> Self {
        Self { scanner }
    }
}

impl EventSubscriber for ScanPlexFolderOnVideoDownloaded {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoDownloadedPayload = serde_json::from_str(payload)?;
        self.scanner
            .scan_folder(&Path::new(&payload.output_dir).join(&payload.folder))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::plex_library_repository::FakePlexLibraryRepository;
    use std::sync::Arc;

    #[test]
    fn it_should_scan_the_folder_in_the_section_containing_it() {
        let plex_repository = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/volume1/media/yarrtube/playlists"),
        );
        let subscriber = ScanPlexFolderOnVideoDownloaded::new(PlexFolderScanner::new(
            "/videos",
            "/volume1/media/yarrtube",
            plex_repository.clone(),
        ));

        let result = handle(
            &subscriber,
            &payload("/videos/playlists/kids", "Some video"),
        );

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.mutations(),
            vec!["scan:19:/volume1/media/yarrtube/playlists/kids/Some video".to_string()]
        );
    }

    #[test]
    fn it_should_skip_a_folder_outside_the_videos_root() {
        let plex_repository = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/volume1/media/yarrtube/playlists"),
        );
        let subscriber = ScanPlexFolderOnVideoDownloaded::new(PlexFolderScanner::new(
            "/videos",
            "/volume1/media/yarrtube",
            plex_repository.clone(),
        ));

        let result = handle(&subscriber, &payload("/elsewhere/kids", "Some video"));

        assert_eq!(result, Ok(()));
        assert_eq!(plex_repository.mutations(), Vec::<String>::new());
    }

    #[test]
    fn it_should_skip_a_folder_no_section_contains() {
        let plex_repository = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/volume1/media/yarrtube/playlists"),
        );
        let subscriber = ScanPlexFolderOnVideoDownloaded::new(PlexFolderScanner::new(
            "/videos",
            "/volume1/media/yarrtube",
            plex_repository.clone(),
        ));

        let result = handle(
            &subscriber,
            &payload("/videos/channels/Some Channel", "Some video"),
        );

        assert_eq!(result, Ok(()));
        assert_eq!(plex_repository.mutations(), Vec::<String>::new());
    }

    #[test]
    fn it_should_fail_if_plex_is_unreachable() {
        let subscriber = ScanPlexFolderOnVideoDownloaded::new(PlexFolderScanner::new(
            "/videos",
            "/volume1/media/yarrtube",
            Arc::new(FakePlexLibraryRepository::failing()),
        ));

        let result = handle(
            &subscriber,
            &payload("/videos/playlists/kids", "Some video"),
        );

        assert_eq!(result, Err("Plex is unreachable".to_string()));
    }

    fn payload(output_dir: &str, folder: &str) -> String {
        serde_json::json!({
            "video_id": "rec1",
            "output_dir": output_dir,
            "folder": folder,
        })
        .to_string()
    }

    fn handle(subscriber: &ScanPlexFolderOnVideoDownloaded, payload: &str) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
