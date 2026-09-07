use crate::domain::playlist::{Playlist, PlaylistError, PlaylistName, YoutubePlaylistId};
use crate::domain::ports::{PlaylistRepository, RepositoryError, SaveOutcome};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::Mutex;

pub struct SqlitePlaylistRepository {
    conn: Mutex<Connection>,
}

impl SqlitePlaylistRepository {
    pub fn new(conn: Connection) -> Result<Self, RepositoryError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS playlists (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| RepositoryError(format!("failed to create playlists table: {e}")))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn row_to_playlist(
        id: String,
        name: String,
        created_at: String,
    ) -> Result<Playlist, RepositoryError> {
        let id = YoutubePlaylistId::new(id)
            .map_err(|e: PlaylistError| RepositoryError(e.to_string()))?;
        let name =
            PlaylistName::new(name).map_err(|e: PlaylistError| RepositoryError(e.to_string()))?;
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| RepositoryError(format!("failed to parse stored created_at: {e}")))?
            .with_timezone(&Utc);
        Ok(Playlist::create(id, name, created_at))
    }

    fn find(&self, id: &YoutubePlaylistId) -> Result<Option<Playlist>, RepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| RepositoryError("database lock poisoned".to_string()))?;
        conn.query_row(
            "SELECT id, name, created_at FROM playlists WHERE id = ?1",
            params![id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|e| RepositoryError(format!("failed to query playlist: {e}")))?
        .map(|(id, name, created_at)| Self::row_to_playlist(id, name, created_at))
        .transpose()
    }
}

fn is_primary_key_violation(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(e, _) if e.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

impl PlaylistRepository for SqlitePlaylistRepository {
    fn save(&self, playlist: &Playlist) -> Result<SaveOutcome, RepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| RepositoryError("database lock poisoned".to_string()))?;
        let result = conn.execute(
            "INSERT INTO playlists (id, name, created_at) VALUES (?1, ?2, ?3)",
            params![
                playlist.id.as_str(),
                playlist.name.as_str(),
                playlist.created_at.to_rfc3339()
            ],
        );
        drop(conn);

        match result {
            Ok(_) => Ok(SaveOutcome::Created(playlist.clone())),
            Err(e) if is_primary_key_violation(&e) => {
                let existing = self.find(&playlist.id)?.ok_or_else(|| {
                    RepositoryError("primary key violation but no existing row found".to_string())
                })?;
                Ok(SaveOutcome::AlreadyExisted(existing))
            }
            Err(e) => Err(RepositoryError(format!("failed to save playlist: {e}"))),
        }
    }

    fn delete(&self, id: &YoutubePlaylistId) -> Result<bool, RepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| RepositoryError("database lock poisoned".to_string()))?;
        let affected = conn
            .execute("DELETE FROM playlists WHERE id = ?1", params![id.as_str()])
            .map_err(|e| RepositoryError(format!("failed to delete playlist: {e}")))?;
        Ok(affected > 0)
    }

    fn list(&self) -> Result<Vec<Playlist>, RepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| RepositoryError("database lock poisoned".to_string()))?;
        let mut stmt = conn
            .prepare("SELECT id, name, created_at FROM playlists ORDER BY rowid ASC")
            .map_err(|e| RepositoryError(format!("failed to prepare list query: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| RepositoryError(format!("failed to list playlists: {e}")))?;

        rows.map(|row| {
            let (id, name, created_at) =
                row.map_err(|e| RepositoryError(format!("failed to read playlist row: {e}")))?;
            Self::row_to_playlist(id, name, created_at)
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> SqlitePlaylistRepository {
        SqlitePlaylistRepository::new(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn playlist(id: &str, name: &str) -> Playlist {
        Playlist::create(
            YoutubePlaylistId::new(id).unwrap(),
            PlaylistName::new(name).unwrap(),
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
    }

    #[test]
    fn save_creates_new_playlist() {
        let repo = repo();
        let playlist = playlist("PL1", "First");

        let outcome = repo.save(&playlist).unwrap();

        assert_eq!(outcome, SaveOutcome::Created(playlist));
    }

    #[test]
    fn save_is_idempotent_on_duplicate_id() {
        let repo = repo();
        let first = playlist("PL1", "First");
        repo.save(&first).unwrap();

        let duplicate = playlist("PL1", "Different Name");
        let outcome = repo.save(&duplicate).unwrap();

        assert_eq!(outcome, SaveOutcome::AlreadyExisted(first));
    }

    #[test]
    fn delete_removes_existing_playlist() {
        let repo = repo();
        let playlist = playlist("PL1", "First");
        repo.save(&playlist).unwrap();

        assert!(repo.delete(&playlist.id).unwrap());
        assert_eq!(repo.list().unwrap(), Vec::new());
    }

    #[test]
    fn delete_reports_not_found_for_missing_playlist() {
        let repo = repo();
        let id = YoutubePlaylistId::new("PL404").unwrap();

        assert!(!repo.delete(&id).unwrap());
    }

    #[test]
    fn list_is_empty_when_no_playlists_saved() {
        let repo = repo();
        assert_eq!(repo.list().unwrap(), Vec::new());
    }

    #[test]
    fn list_returns_all_saved_playlists() {
        let repo = repo();
        let first = playlist("PL1", "First");
        let second = playlist("PL2", "Second");
        repo.save(&first).unwrap();
        repo.save(&second).unwrap();

        let listed = repo.list().unwrap();

        assert_eq!(listed, vec![first, second]);
    }
}
