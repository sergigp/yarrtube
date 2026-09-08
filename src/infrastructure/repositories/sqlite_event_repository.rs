use crate::domain::event::DomainEvent;
use crate::infrastructure::repositories::system_clock::Clock;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};

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
    println!(
        "[events] published {} (id={}): {}",
        event.event_type(),
        conn.last_insert_rowid(),
        event.payload()
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
        conn.execute(
            "UPDATE events SET status = 'done', updated_at = ?2 WHERE id = ?1",
            params![id, self.clock.now().to_rfc3339()],
        )
        .context("failed to mark event done")?;
        println!("[events] event {id} done");
        Ok(())
    }

    fn mark_failed_or_retry(&self, id: i64, error: &str) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let retries: i64 = conn
            .query_row(
                "SELECT retries FROM events WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .context("failed to read event retries")?;
        let retries = retries + 1;
        let status = if retries >= MAX_ATTEMPTS {
            "failed"
        } else {
            "pending"
        };
        conn.execute(
            "UPDATE events SET status = ?2, retries = ?3, updated_at = ?4, last_error = ?5 WHERE id = ?1",
            params![id, status, retries, self.clock.now().to_rfc3339(), error],
        )
        .context("failed to update event after failure")?;
        if status == "failed" {
            println!("[events] event {id} failed permanently after {retries} attempts: {error}");
        } else {
            println!(
                "[events] event {id} failed (attempt {retries}/{MAX_ATTEMPTS}), will retry: {error}"
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
