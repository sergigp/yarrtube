use crate::domain::plex_label::CategoryPlaylist;
use crate::domain::preferences::{PlaylistOrder, PlaylistWatched, PlexPlaylistSettings};
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
                "SELECT section_id, label, rating_key, title, watched, playlist_order
                 FROM category_playlists
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
            "INSERT OR REPLACE INTO category_playlists
                 (section_id, label, rating_key, title, watched, playlist_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                playlist.section_id,
                playlist.label,
                playlist.rating_key,
                playlist.title,
                playlist.options.map(|options| options.watched.as_str()),
                playlist.options.map(|options| options.order.as_str()),
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
            options: Self::options(row.get(4)?, row.get(5)?),
        })
    }

    /// The options both columns hold; `None` when either is missing or no
    /// longer valid, so the playlist is replaced.
    fn options(watched: Option<String>, order: Option<String>) -> Option<PlexPlaylistSettings> {
        Some(PlexPlaylistSettings {
            watched: PlaylistWatched::new(&watched?).ok()?,
            order: PlaylistOrder::new(&order?).ok()?,
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

    #[test]
    fn it_should_key_playlists_by_label_ignoring_case() {
        let db = TestDatabase::new();
        let repository = SqliteCategoryPlaylistRepository::new(db.database());
        repository.save(&playlist("19", "Chess", "pl1")).unwrap();

        repository.save(&playlist("19", "chess", "pl2")).unwrap();

        assert_eq!(
            repository.list().unwrap(),
            vec![playlist("19", "chess", "pl2")]
        );
    }

    #[test]
    fn it_should_round_trip_a_playlists_options() {
        let db = TestDatabase::new();
        let repository = SqliteCategoryPlaylistRepository::new(db.database());
        let unwatched_oldest_first = CategoryPlaylist {
            options: Some(PlexPlaylistSettings {
                watched: PlaylistWatched::Drop,
                order: PlaylistOrder::OldestFirst,
            }),
            ..playlist("19", "Chess", "pl1")
        };

        repository.save(&unwatched_oldest_first).unwrap();

        assert_eq!(repository.list().unwrap(), vec![unwatched_oldest_first]);
    }

    #[test]
    fn it_should_read_a_playlist_without_options_as_unknown() {
        let db = TestDatabase::new();
        let repository = SqliteCategoryPlaylistRepository::new(db.database());
        let unknown = CategoryPlaylist {
            options: None,
            ..playlist("19", "Chess", "pl1")
        };

        repository.save(&unknown).unwrap();

        assert_eq!(repository.list().unwrap(), vec![unknown]);
    }

    fn playlist(section_id: &str, label: &str, rating_key: &str) -> CategoryPlaylist {
        CategoryPlaylist {
            section_id: section_id.to_string(),
            label: label.to_string(),
            rating_key: rating_key.to_string(),
            title: label.to_string(),
            options: None,
        }
    }
}
