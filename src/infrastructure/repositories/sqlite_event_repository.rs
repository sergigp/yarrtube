use crate::domain::event::DomainEvent;
use crate::infrastructure::repositories::system_clock::Clock;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

const MAX_ATTEMPTS: i64 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedEvent {
    pub id: i64,
    pub event_type: String,
    pub payload: String,
}

pub trait EventPublisher: Send + Sync {
    fn publish(&self, event: &DomainEvent) -> anyhow::Result<()>;
}

pub trait EventRepository: Send + Sync {
    fn insert_pending(&self, event: &DomainEvent) -> anyhow::Result<()>;
    fn list_eligible(&self) -> anyhow::Result<Vec<PersistedEvent>>;
    fn mark_done(&self, id: i64) -> anyhow::Result<()>;
    fn mark_failed_or_retry(&self, id: i64, error: &str) -> anyhow::Result<()>;
}

/// Creates the `events` table if it doesn't exist yet. Exposed so
/// `SqlitePlaylistRepository` can ensure it exists too, since it writes to
/// this table directly to keep a playlist insert and its event in one
/// transaction (see `insert_pending_row`).
pub(crate) fn create_events_table(conn: &Connection) -> anyhow::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            payload TEXT NOT NULL,
            status TEXT NOT NULL,
            retries INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_error TEXT
        )",
        [],
    )
    .context("failed to create events table")?;
    Ok(())
}

fn create_domain_events_dead_letter_table(conn: &Connection) -> anyhow::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS domain_events_dead_letter (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            original_event_id INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            payload TEXT NOT NULL,
            retries INTEGER NOT NULL,
            last_error TEXT,
            created_at TEXT NOT NULL,
            failed_at TEXT NOT NULL
        )",
        [],
    )
    .context("failed to create domain_events_dead_letter table")?;
    Ok(())
}

/// Inserts a pending event row against any `Connection` (a plain connection
/// or a `Transaction`, which derefs to one) so callers writing to another
/// table can include this write in the same transaction.
pub(crate) fn insert_pending_row(
    conn: &Connection,
    event: &DomainEvent,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO events (event_type, payload, status, retries, created_at, updated_at, last_error)
         VALUES (?1, ?2, 'pending', 0, ?3, ?3, NULL)",
        params![
            event.event_type(),
            event.payload().to_string(),
            now.to_rfc3339()
        ],
    )
    .context("failed to insert event")?;
    info!(
        event_id = conn.last_insert_rowid(),
        event_type = event.event_type(),
        payload = %event.payload(),
        "published event"
    );
    Ok(())
}

pub struct SqliteEventRepository {
    conn: Arc<Mutex<Connection>>,
    clock: Arc<dyn Clock>,
}

impl SqliteEventRepository {
    pub fn new(conn: Arc<Mutex<Connection>>, clock: Arc<dyn Clock>) -> anyhow::Result<Self> {
        {
            let guard = conn
                .lock()
                .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
            create_events_table(&guard)?;
            create_domain_events_dead_letter_table(&guard)?;
        }
        Ok(Self { conn, clock })
    }
}

impl EventPublisher for SqliteEventRepository {
    fn publish(&self, event: &DomainEvent) -> anyhow::Result<()> {
        self.insert_pending(event)
    }
}

impl EventRepository for SqliteEventRepository {
    fn insert_pending(&self, event: &DomainEvent) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        insert_pending_row(&conn, event, self.clock.now())
    }

    fn list_eligible(&self) -> anyhow::Result<Vec<PersistedEvent>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut stmt = conn
            .prepare("SELECT id, event_type, payload FROM events WHERE status = 'pending' ORDER BY id ASC")
            .context("failed to prepare list-eligible-events query")?;
        let rows = stmt
            .query_map([], |row| {
                Ok(PersistedEvent {
                    id: row.get(0)?,
                    event_type: row.get(1)?,
                    payload: row.get(2)?,
                })
            })
            .context("failed to list eligible events")?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("failed to read event row")
    }

    fn mark_done(&self, id: i64) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute("DELETE FROM events WHERE id = ?1", params![id])
            .context("failed to mark event done")?;
        info!(event_id = id, "event done");
        Ok(())
    }

    fn mark_failed_or_retry(&self, id: i64, error: &str) -> anyhow::Result<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let (event_type, payload, created_at, retries): (String, String, String, i64) = conn
            .query_row(
                "SELECT event_type, payload, created_at, retries FROM events WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .context("failed to read event before failure")?;
        let retries = retries + 1;

        let now = self.clock.now();

        if retries >= MAX_ATTEMPTS {
            let tx = conn
                .transaction()
                .context("failed to start dead-letter transaction")?;
            tx.execute(
                "INSERT INTO domain_events_dead_letter (original_event_id, event_type, payload, retries, last_error, created_at, failed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![id, event_type, payload, retries, error, created_at, now.to_rfc3339()],
            )
            .context("failed to insert event into dead letter")?;
            tx.execute("DELETE FROM events WHERE id = ?1", params![id])
                .context("failed to delete event after moving to dead letter")?;
            tx.commit()
                .context("failed to commit dead-letter transaction")?;
            println!("[events] event {id} failed permanently after {retries} attempts: {error}");
        } else {
            conn.execute(
                "UPDATE events SET status = 'pending', retries = ?2, updated_at = ?3, last_error = ?4 WHERE id = ?1",
                params![id, retries, now.to_rfc3339(), error],
            )
            .context("failed to update event after failure")?;
            warn!(
                event_id = id,
                retries,
                max_attempts = MAX_ATTEMPTS,
                error,
                "event failed, retrying"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeEventPublisher {
    pub(crate) published: Mutex<Vec<DomainEvent>>,
}

#[cfg(test)]
impl EventPublisher for FakeEventPublisher {
    fn publish(&self, event: &DomainEvent) -> anyhow::Result<()> {
        self.published.lock().unwrap().push(event.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::system_clock::FixedClock;
    use rusqlite::OptionalExtension;

    fn repo() -> SqliteEventRepository {
        SqliteEventRepository::new(
            Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
            Arc::new(FixedClock(DateTime::<Utc>::from_timestamp(0, 0).unwrap())),
        )
        .unwrap()
    }

    fn event() -> DomainEvent {
        DomainEvent::PlaylistCreated {
            playlist_id: "PL1".to_string(),
        }
    }

    #[test]
    fn it_should_create_a_queryable_empty_dead_letter_table() {
        let repo = repo();

        let conn = repo.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT * FROM domain_events_dead_letter")
            .unwrap();
        let rows = stmt.query_map([], |_| Ok(())).unwrap();

        assert_eq!(rows.count(), 0);
    }

    #[test]
    fn it_should_list_an_inserted_event_as_eligible() {
        let repo = repo();

        repo.insert_pending(&event()).unwrap();

        let eligible = repo.list_eligible().unwrap();
        assert_eq!(eligible.len(), 1);
        assert_eq!(eligible[0].event_type, "playlist_created");
        assert_eq!(eligible[0].payload, event().payload().to_string());
    }

    #[test]
    fn it_should_no_longer_list_an_event_as_eligible_once_marked_done() {
        let repo = repo();
        repo.insert_pending(&event()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;

        repo.mark_done(id).unwrap();

        assert!(repo.list_eligible().unwrap().is_empty());
        let conn = repo.conn.lock().unwrap();
        let found: Option<i64> = conn
            .query_row("SELECT id FROM events WHERE id = ?1", params![id], |row| {
                row.get(0)
            })
            .optional()
            .unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn it_should_keep_a_failed_event_pending_and_increment_retries_below_the_limit() {
        let repo = repo();
        repo.insert_pending(&event()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;

        repo.mark_failed_or_retry(id, "boom").unwrap();

        assert_eq!(repo.list_eligible().unwrap().len(), 1);
    }

    #[test]
    fn it_should_drop_the_event_after_the_fifth_failed_attempt() {
        let repo = repo();
        repo.insert_pending(&event()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;

        for _ in 0..5 {
            repo.mark_failed_or_retry(id, "boom").unwrap();
        }

        assert!(repo.list_eligible().unwrap().is_empty());
    }

    #[test]
    fn it_should_move_the_event_to_the_dead_letter_table_after_the_fifth_failed_attempt() {
        let repo = repo();
        repo.insert_pending(&event()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;

        for _ in 0..5 {
            repo.mark_failed_or_retry(id, "boom").unwrap();
        }

        assert!(repo.list_eligible().unwrap().is_empty());
        let conn = repo.conn.lock().unwrap();
        let found: Option<i64> = conn
            .query_row("SELECT id FROM events WHERE id = ?1", params![id], |row| {
                row.get(0)
            })
            .optional()
            .unwrap();
        assert!(found.is_none());

        let (original_event_id, event_type, payload, retries, last_error): (
            i64,
            String,
            String,
            i64,
            String,
        ) = conn
            .query_row(
                "SELECT original_event_id, event_type, payload, retries, last_error FROM domain_events_dead_letter",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .unwrap();
        assert_eq!(original_event_id, id);
        assert_eq!(event_type, "playlist_created");
        assert_eq!(payload, event().payload().to_string());
        assert_eq!(retries, 5);
        assert_eq!(last_error, "boom");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM domain_events_dead_letter",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn it_should_stay_eligible_after_the_fourth_failed_attempt() {
        let repo = repo();
        repo.insert_pending(&event()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;

        for _ in 0..4 {
            repo.mark_failed_or_retry(id, "boom").unwrap();
        }

        assert_eq!(repo.list_eligible().unwrap().len(), 1);
    }
}
