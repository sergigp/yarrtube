use crate::domain::services::{PlexFolderScanner, PlexFolderScannerApi};
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;
use std::path::Path;

/// The event's `folder` (the season folder) is ignored: the whole show is
/// scanned.
#[derive(Debug, Deserialize)]
struct VideoDownloadedPayload {
    output_dir: String,
}

/// Reacts to `VideoDownloaded` by asking Plex to scan the video's show
/// folder.
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
        self.scanner.scan_folder(Path::new(&payload.output_dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tasks::log_capture::captured_log_messages;
    use crate::infrastructure::repositories::plex_library_repository::FakePlexLibraryRepository;
    use std::sync::Arc;

    #[test]
    fn it_should_scan_the_show_folder_in_every_section_containing_it() {
        let plex_repository = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/volume1/media/yarrtube/channels")
                .with_location("20", "/volume1/media/yarrtube/playlists"),
        );
        let subscriber = ScanPlexFolderOnVideoDownloaded::new(PlexFolderScanner::new(
            "/videos",
            "/volume1/media/yarrtube",
            plex_repository.clone(),
        ));

        let result = handle(
            &subscriber,
            &payload("/videos/channels/Some Channel", "Season 2026"),
        );

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex_repository.mutations(),
            vec!["scan:19:/volume1/media/yarrtube/channels/Some Channel".to_string()]
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
    fn it_should_skip_the_scan_when_no_section_contains_the_show_folder() {
        let plex_repository = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/volume1/media/yarrtube/playlists"),
        );
        let subscriber = ScanPlexFolderOnVideoDownloaded::new(PlexFolderScanner::new(
            "/videos",
            "/volume1/media/yarrtube",
            plex_repository.clone(),
        ));

        let mut result = Ok(());
        let logs = captured_log_messages(|| {
            result = handle(
                &subscriber,
                &payload("/videos/channels/Some Channel", "Season 2026"),
            );
        });

        assert_eq!(result, Ok(()));
        assert_eq!(plex_repository.mutations(), Vec::<String>::new());
        assert_eq!(
            logs,
            vec!["no Plex section contains the folder, skipping Plex scan".to_string()]
        );
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
