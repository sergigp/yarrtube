use crate::domain::playlist::PlaylistId;
use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
use crate::domain::shared::Quality;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::Mutex;

pub trait PlaylistRepository: Send + Sync {
    fn find(&self, id: &PlaylistId) -> anyhow::Result<Option<Playlist>>;
    fn insert(&self, playlist: &Playlist) -> anyhow::Result<()>;
    fn delete(&self, id: &PlaylistId) -> anyhow::Result<()>;
    fn list(&self) -> anyhow::Result<Vec<Playlist>>;
}

pub struct SqlitePlaylistRepository {
    conn: Mutex<Connection>,
}

impl SqlitePlaylistRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }
}

impl PlaylistRepository for SqlitePlaylistRepository {
    fn find(&self, id: &PlaylistId) -> anyhow::Result<Option<Playlist>> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(playlist_id = %id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.query_row(
            "SELECT id, name, path, quality, kind, created_at FROM playlists WHERE id = ?1",
            params![id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .inspect_err(|e| tracing::error!(playlist_id = %id, error = %e, "failed to query playlist"))
        .context("failed to query playlist")?
        .map(|(id, name, path, quality, kind, created_at)| {
            Self::row_to_playlist(id, name, path, quality, kind, created_at)
        })
        .transpose()
    }

    fn insert(&self, playlist: &Playlist) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(playlist_id = %playlist.id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "INSERT INTO playlists (id, name, path, quality, kind, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                playlist.id.as_str(),
                playlist.name.as_str(),
                playlist.path.as_str(),
                playlist.quality.as_str(),
                playlist.kind.as_str(),
                playlist.created_at.to_rfc3339()
            ],
        )
        .inspect_err(|e| tracing::error!(playlist_id = %playlist.id, error = %e, "failed to insert playlist"))
        .context("failed to insert playlist")?;
        Ok(())
    }

    fn delete(&self, id: &PlaylistId) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(playlist_id = %id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute("DELETE FROM playlists WHERE id = ?1", params![id.as_str()])
            .inspect_err(
                |e| tracing::error!(playlist_id = %id, error = %e, "failed to delete playlist"),
            )
            .context("failed to delete playlist")?;
        Ok(())
    }

    fn list(&self) -> anyhow::Result<Vec<Playlist>> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!("database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, path, quality, kind, created_at FROM playlists ORDER BY rowid ASC",
            )
            .inspect_err(|e| tracing::error!(error = %e, "failed to prepare list query"))
            .context("failed to prepare list query")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .inspect_err(|e| tracing::error!(error = %e, "failed to list playlists"))
            .context("failed to list playlists")?;

        rows.map(|row| {
            let (id, name, path, quality, kind, created_at) = row
                .inspect_err(|e| tracing::error!(error = %e, "failed to read playlist row"))
                .context("failed to read playlist row")?;
            Self::row_to_playlist(id, name, path, quality, kind, created_at)
        })
        .collect()
    }
}

impl SqlitePlaylistRepository {
    #[allow(clippy::too_many_arguments)]
    fn row_to_playlist(
        id: String,
        name: String,
        path: String,
        quality: String,
        kind: String,
        created_at: String,
    ) -> anyhow::Result<Playlist> {
        let id = PlaylistId::new(id)?;
        let name = PlaylistName::new(name)?;
        let path = PlaylistPath::new(path)?;
        let quality = Quality::new(quality)?;
        let kind = PlaylistKind::new(kind)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .context("failed to parse stored created_at")?
            .with_timezone(&Utc);
        Ok(Playlist::create(id, name, path, quality, kind, created_at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> SqlitePlaylistRepository {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::shared::sqlite_migrations::apply(&mut conn).unwrap();
        SqlitePlaylistRepository::new(conn)
    }

    fn playlist(id: &str, name: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new(name).unwrap(),
            PlaylistPath::new("my-playlist").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
    }

    #[test]
    fn it_should_return_none_when_the_playlist_does_not_exist() {
        let repo = repo();
        let id = PlaylistId::new("PL404").unwrap();

        assert_eq!(repo.find(&id).unwrap(), None);
    }

    #[test]
    fn it_should_return_the_playlist_after_inserting_it() {
        let repo = repo();
        let playlist = playlist("PL1", "First");

        repo.insert(&playlist).unwrap();

        assert_eq!(repo.find(&playlist.id).unwrap(), Some(playlist));
    }

    #[test]
    fn it_should_fail_when_inserting_a_duplicate_id() {
        let repo = repo();
        let playlist = playlist("PL1", "First");
        repo.insert(&playlist).unwrap();

        assert!(repo.insert(&playlist).is_err());
    }

    #[test]
    fn it_should_remove_an_existing_playlist() {
        let repo = repo();
        let playlist = playlist("PL1", "First");
        repo.insert(&playlist).unwrap();

        repo.delete(&playlist.id).unwrap();

        assert_eq!(repo.find(&playlist.id).unwrap(), None);
    }

    #[test]
    fn it_should_succeed_when_deleting_a_missing_playlist() {
        let repo = repo();
        let id = PlaylistId::new("PL404").unwrap();

        assert!(repo.delete(&id).is_ok());
    }

    #[test]
    fn it_should_return_an_empty_list_when_no_playlists_are_saved() {
        let repo = repo();

        assert_eq!(repo.list().unwrap(), Vec::new());
    }

    #[test]
    fn it_should_return_all_saved_playlists() {
        let repo = repo();
        let first = playlist("PL1", "First");
        let second = playlist("PL2", "Second");
        repo.insert(&first).unwrap();
        repo.insert(&second).unwrap();

        assert_eq!(repo.list().unwrap(), vec![first, second]);
    }
}
