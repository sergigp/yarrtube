use crate::domain::plex::plex_folder_path;
use crate::infrastructure::repositories::plex_library_repository::PlexLibraryRepository;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Asks Plex to scan a newly downloaded video's show folder in every library
/// section that contains it, so the episode is imported right away instead
/// of whenever Plex next scans the library. Sections are discovered, not
/// configured. Plex sees yarrtube's `videos_root` at `plex_videos_root`.
#[derive(Clone)]
pub struct PlexFolderScanner {
    videos_root: PathBuf,
    plex_videos_root: String,
    plex_library_repository: Arc<dyn PlexLibraryRepository>,
}

impl PlexFolderScanner {
    pub fn new(
        videos_root: impl Into<PathBuf>,
        plex_videos_root: impl Into<String>,
        plex_library_repository: Arc<dyn PlexLibraryRepository>,
    ) -> Self {
        Self {
            videos_root: videos_root.into(),
            plex_videos_root: plex_videos_root.into(),
            plex_library_repository,
        }
    }
}

pub trait PlexFolderScannerApi: Send + Sync {
    /// Scans `folder` (yarrtube-side path) in every section whose
    /// locations contain its Plex-side path. A folder outside the videos
    /// root, or contained by no section, is logged and skipped.
    /// Fails when Plex can't be reached, so the caller can retry.
    fn scan_folder(&self, folder: &Path) -> anyhow::Result<()>;
}

impl PlexFolderScannerApi for PlexFolderScanner {
    fn scan_folder(&self, folder: &Path) -> anyhow::Result<()> {
        let Some(plex_path) = plex_folder_path(&self.videos_root, &self.plex_videos_root, folder)
        else {
            debug!(folder = ?folder, "folder is outside the videos root, skipping Plex scan");
            return Ok(());
        };
        let section_ids = self.sections_containing(&plex_path)?;
        if section_ids.is_empty() {
            warn!(path = %plex_path, "no Plex section contains the folder, skipping Plex scan");
            return Ok(());
        }
        section_ids
            .iter()
            .try_for_each(|section_id| self.scan(section_id, &plex_path))
    }
}

impl PlexFolderScanner {
    /// The sections whose locations contain `plex_path`.
    fn sections_containing(&self, plex_path: &str) -> anyhow::Result<Vec<String>> {
        Ok(self
            .plex_library_repository
            .list_sections()?
            .into_iter()
            .filter(|section| section.contains(plex_path))
            .map(|section| section.id)
            .collect())
    }

    fn scan(&self, section_id: &str, plex_path: &str) -> anyhow::Result<()> {
        self.plex_library_repository
            .scan_path(section_id, plex_path)?;
        info!(section = %section_id, path = %plex_path, "requested Plex scan of downloaded video's folder");
        Ok(())
    }
}
