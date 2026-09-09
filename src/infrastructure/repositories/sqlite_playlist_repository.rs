use crate::domain::event::DomainEvent;
use crate::domain::playlist::{Playlist, PlaylistName, Quality};
use crate::domain::shared::PlaylistId;
use crate::infrastructure::repositories::sqlite_event_repository::{
    create_events_table, insert_pending_row,
};
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::{Arc, Mutex};

pub trait PlaylistRepository: Send + Sync {
    fn find(&self, id: &PlaylistId) -> anyhow::Result<Option<Playlist>>;
    /// Inserts the playlist and a pending event row in the same SQLite
    /// transaction, so a publish can never be lost even if the process dies
    /// right after this call returns.
    fn insert_with_event(
        &self,
        playlist: &Playlist,
        event: &DomainEvent,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()>;
    /// Deletes the playlist and inserts a pending event row in the same
    /// SQLite transaction.
    fn delete_with_event(
        &self,
        id: &PlaylistId,
        event: &DomainEvent,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()>;
    fn list(&self) -> anyhow::Result<Vec<Playlist>>;
}

pub struct SqlitePlaylistRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqlitePlaylistRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> anyhow::Result<Self> {
        {
            let guard = conn
                .lock()
                .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
            guard
                .execute(
                    "CREATE TABLE IF NOT EXISTS playlists (
                        id TEXT PRIMARY KEY,
                        name TEXT NOT NULL,
                        quality TEXT NOT NULL,
                        created_at TEXT NOT NULL
                    )",
                    [],
                )
                .context("failed to create playlists table")?;
            // Ensured here too (not just by SqliteEventRepository::new) so
            // insert_with_event/delete_with_event can write to it within
            // their own transaction regardless of construction order.
            create_events_table(&guard)?;
        }
        Ok(Self { conn })
    }

    fn row_to_playlist(
        id: String,
        name: String,
        quality: String,
        created_at: String,
    ) -> anyhow::Result<Playlist> {
        let id = PlaylistId::new(id)?;
        let name = PlaylistName::new(name)?;
        let quality = Quality::new(quality)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .context("failed to parse stored created_at")?
            .with_timezone(&Utc);
        Ok(Playlist::create(id, name, quality, created_at))
    }
}

impl PlaylistRepository for SqlitePlaylistRepository {
    fn find(&self, id: &PlaylistId) -> anyhow::Result<Option<Playlist>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.query_row(
            "SELECT id, name, quality, created_at FROM playlists WHERE id = ?1",
            params![id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .context("failed to query playlist")?
        .map(|(id, name, quality, created_at)| Self::row_to_playlist(id, name, quality, created_at))
        .transpose()
    }

    fn insert_with_event(
        &self,
        playlist: &Playlist,
        event: &DomainEvent,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let tx = conn.transaction().context("failed to start transaction")?;
        tx.execute(
            "INSERT INTO playlists (id, name, quality, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                playlist.id.as_str(),
                playlist.name.as_str(),
                playlist.quality.as_str(),
                playlist.created_at.to_rfc3339()
            ],
        )
        .context("failed to insert playlist")?;
        insert_pending_row(&tx, event, now).context("failed to insert event")?;
        tx.commit().context("failed to commit transaction")?;
        Ok(())
    }

    fn delete_with_event(
        &self,
        id: &PlaylistId,
        event: &DomainEvent,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let tx = conn.transaction().context("failed to start transaction")?;
        tx.execute("DELETE FROM playlists WHERE id = ?1", params![id.as_str()])
            .context("failed to delete playlist")?;
        insert_pending_row(&tx, event, now).context("failed to insert event")?;
        tx.commit().context("failed to commit transaction")?;
        Ok(())
    }

    fn list(&self) -> anyhow::Result<Vec<Playlist>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut stmt = conn
            .prepare("SELECT id, name, quality, created_at FROM playlists ORDER BY rowid ASC")
            .context("failed to prepare list query")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .context("failed to list playlists")?;

        rows.map(|row| {
            let (id, name, quality, created_at) = row.context("failed to read playlist row")?;
            Self::row_to_playlist(id, name, quality, created_at)
        })
        .collect()
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakePlaylistRepository {
    playlists: Mutex<Vec<Playlist>>,
    /// Events written transactionally alongside a playlist insert/delete —
    /// this is what production code actually records them into (see
    /// `insert_with_event`/`delete_with_event` above), not a separately
    /// injected `EventPublisher`.
    pub(crate) transactional_events: Mutex<Vec<DomainEvent>>,
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

    fn insert_with_event(
        &self,
        playlist: &Playlist,
        event: &DomainEvent,
        _now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        self.transactional_events
            .lock()
            .unwrap()
            .push(event.clone());
        self.playlists.lock().unwrap().push(playlist.clone());
        Ok(())
    }

    fn delete_with_event(
        &self,
        id: &PlaylistId,
        event: &DomainEvent,
        _now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        self.transactional_events
            .lock()
            .unwrap()
            .push(event.clone());
        self.playlists.lock().unwrap().retain(|p| p.id != *id);
        Ok(())
    }

    fn list(&self) -> anyhow::Result<Vec<Playlist>> {
        Ok(self.playlists.lock().unwrap().clone())
    }
}

#[cfg(test)]
impl SqlitePlaylistRepository {
    /// Not part of the `PlaylistRepository` port — production code always
    /// goes through `insert_with_event`/`delete_with_event` — but useful for
    /// exercising the bare `playlists` table in isolation below.
    fn insert(&self, playlist: &Playlist) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "INSERT INTO playlists (id, name, quality, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                playlist.id.as_str(),
                playlist.name.as_str(),
                playlist.quality.as_str(),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> SqlitePlaylistRepository {
        SqlitePlaylistRepository::new(Arc::new(Mutex::new(Connection::open_in_memory().unwrap())))
            .unwrap()
    }

    fn playlist(id: &str, name: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new(name).unwrap(),
            Quality::High,
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
    }

    fn event_count(repo: &SqlitePlaylistRepository) -> i64 {
        let conn = repo.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .unwrap()
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

    #[test]
    fn it_should_commit_the_playlist_and_the_event_together() {
        let repo = repo();
        let playlist = playlist("PL1", "First");
        let event = DomainEvent::PlaylistCreated {
            playlist_id: "PL1".to_string(),
        };

        repo.insert_with_event(&playlist, &event, playlist.created_at)
            .unwrap();

        assert_eq!(repo.find(&playlist.id).unwrap(), Some(playlist));
        assert_eq!(event_count(&repo), 1);
    }

    #[test]
    fn it_should_not_insert_an_event_when_the_playlist_insert_fails() {
        let repo = repo();
        let playlist = playlist("PL1", "First");
        repo.insert(&playlist).unwrap();
        let event = DomainEvent::PlaylistCreated {
            playlist_id: "PL1".to_string(),
        };

        let result = repo.insert_with_event(&playlist, &event, playlist.created_at);

        assert!(result.is_err());
        assert_eq!(event_count(&repo), 0);
    }

    #[test]
    fn it_should_commit_the_playlist_deletion_and_the_event_together() {
        let repo = repo();
        let playlist = playlist("PL1", "First");
        repo.insert(&playlist).unwrap();
        let event = DomainEvent::PlaylistDeleted {
            playlist_id: "PL1".to_string(),
        };

        repo.delete_with_event(&playlist.id, &event, playlist.created_at)
            .unwrap();

        assert_eq!(repo.find(&playlist.id).unwrap(), None);
        assert_eq!(event_count(&repo), 1);
    }
}
