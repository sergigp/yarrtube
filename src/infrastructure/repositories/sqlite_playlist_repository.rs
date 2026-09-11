use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
use crate::domain::shared::{PlaylistId, Quality};
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
    pub fn new(conn: Connection) -> anyhow::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS playlists (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                quality TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )
        .context("failed to create playlists table")?;
        match conn.execute(
            "ALTER TABLE playlists ADD COLUMN kind TEXT NOT NULL DEFAULT 'youtube_linked'",
            [],
        ) {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(_, Some(message)))
                if message.contains("duplicate column name") => {}
            Err(e) => return Err(e).context("failed to add kind column to playlists table"),
        }
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

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

impl PlaylistRepository for SqlitePlaylistRepository {
    fn find(&self, id: &PlaylistId) -> anyhow::Result<Option<Playlist>> {
        let conn = self
            .conn
            .lock()
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
        .context("failed to insert playlist")?;
        Ok(())
    }

    fn delete(&self, id: &PlaylistId) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute("DELETE FROM playlists WHERE id = ?1", params![id.as_str()])
            .context("failed to delete playlist")?;
        Ok(())
    }

    fn list(&self) -> anyhow::Result<Vec<Playlist>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, path, quality, kind, created_at FROM playlists ORDER BY rowid ASC",
            )
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
            .context("failed to list playlists")?;

        rows.map(|row| {
            let (id, name, path, quality, kind, created_at) =
                row.context("failed to read playlist row")?;
            Self::row_to_playlist(id, name, path, quality, kind, created_at)
        })
        .collect()
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakePlaylistRepository {
    playlists: Mutex<Vec<Playlist>>,
}

#[cfg(test)]
impl PlaylistRepository for FakePlaylistRepository {
    fn find(&self, id: &PlaylistId) -> anyhow::Result<Option<Playlist>> {
        Ok(self
            .playlists
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.id == *id)
            .cloned())
    }

    fn insert(&self, playlist: &Playlist) -> anyhow::Result<()> {
        self.playlists.lock().unwrap().push(playlist.clone());
        Ok(())
    }

    fn delete(&self, id: &PlaylistId) -> anyhow::Result<()> {
        self.playlists.lock().unwrap().retain(|p| p.id != *id);
        Ok(())
    }

    fn list(&self) -> anyhow::Result<Vec<Playlist>> {
        Ok(self.playlists.lock().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> SqlitePlaylistRepository {
        SqlitePlaylistRepository::new(Connection::open_in_memory().unwrap()).unwrap()
    }

    #[test]
    fn it_should_be_idempotent_when_constructed_twice_against_an_already_migrated_database() {
        let db_path = std::env::temp_dir().join(format!(
            "yarrtube-playlist-repo-migration-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        SqlitePlaylistRepository::new(Connection::open(&db_path).unwrap()).unwrap();
        let result = SqlitePlaylistRepository::new(Connection::open(&db_path).unwrap());

        assert!(result.is_ok());
        std::fs::remove_file(&db_path).unwrap();
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
