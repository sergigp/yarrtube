use crate::domain::plex_label::CategoryPlaylist;
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use rusqlite::params;

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
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(
                "SELECT section_id, label, rating_key, title FROM category_playlists
                 ORDER BY section_id, label",
            )
            .context("failed to prepare category playlists query")?;
        stmt.query_map([], Self::read_row)
            .inspect_err(|e| tracing::error!(error = %e, "failed to list category playlists"))
            .context("failed to list category playlists")?
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to read category playlist row")
    }

    fn save(&self, playlist: &CategoryPlaylist) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "INSERT OR REPLACE INTO category_playlists (section_id, label, rating_key, title)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                playlist.section_id,
                playlist.label,
                playlist.rating_key,
                playlist.title
            ],
        )
        .inspect_err(|e| tracing::error!(label = %playlist.label, error = %e, "failed to save category playlist"))
        .context("failed to save category playlist")
        .map(drop)
    }

    fn delete(&self, playlist: &CategoryPlaylist) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "DELETE FROM category_playlists WHERE section_id = ?1 AND label = ?2",
            params![playlist.section_id, playlist.label],
        )
        .inspect_err(|e| tracing::error!(label = %playlist.label, error = %e, "failed to delete category playlist"))
        .context("failed to delete category playlist")
        .map(drop)
    }
}

impl SqliteCategoryPlaylistRepository {
    fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CategoryPlaylist> {
        Ok(CategoryPlaylist {
            section_id: row.get(0)?,
            label: row.get(1)?,
            rating_key: row.get(2)?,
            title: row.get(3)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;

    #[test]
    fn it_should_save_list_and_delete_category_playlists() {
        let db = TestDatabase::new();
        let repository = SqliteCategoryPlaylistRepository::new(db.database());
        repository.save(&playlist("21", "Chess", "pl2")).unwrap();
        repository.save(&playlist("19", "Chess", "pl1")).unwrap();
        repository.save(&playlist("19", "Funny", "pl3")).unwrap();

        repository.delete(&playlist("19", "Funny", "pl3")).unwrap();

        assert_eq!(
            repository.list().unwrap(),
            vec![
                playlist("19", "Chess", "pl1"),
                playlist("21", "Chess", "pl2")
            ]
        );
    }

    fn playlist(section_id: &str, label: &str, rating_key: &str) -> CategoryPlaylist {
        CategoryPlaylist {
            section_id: section_id.to_string(),
            label: label.to_string(),
            rating_key: rating_key.to_string(),
            title: label.to_string(),
        }
    }
}
