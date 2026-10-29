use crate::domain::plex_label::CategoryPlaylist;
use crate::infrastructure::shared::sqlite_connection::Database;

/// The smart playlists yarrtube created in Plex, keyed by section and label
/// (ignoring case).
pub trait CategoryPlaylistRepository: Send + Sync {
    fn list(&self) -> anyhow::Result<Vec<CategoryPlaylist>>;
    /// Inserts or replaces the playlist of its section and label.
    fn save(&self, playlist: &CategoryPlaylist) -> anyhow::Result<()>;
    fn delete(&self, playlist: &CategoryPlaylist) -> anyhow::Result<()>;
}

pub struct SqliteCategoryPlaylistRepository {
    db: Database,
}

impl SqliteCategoryPlaylistRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl CategoryPlaylistRepository for SqliteCategoryPlaylistRepository {
    fn list(&self) -> anyhow::Result<Vec<CategoryPlaylist>> {
        Ok(Vec::new())
    }

    fn save(&self, _playlist: &CategoryPlaylist) -> anyhow::Result<()> {
        Ok(())
    }

    fn delete(&self, _playlist: &CategoryPlaylist) -> anyhow::Result<()> {
        Ok(())
    }
}
