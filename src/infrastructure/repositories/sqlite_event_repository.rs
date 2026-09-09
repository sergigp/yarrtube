use crate::domain::event::{DeadLetteredEvent, DomainEvent, ScheduledEvent};
use crate::infrastructure::repositories::system_clock::Clock;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params};
use std::sync::{Arc, Mutex};
use tracing::info;

pub trait EventPublisher: Send + Sync {
    fn publish(&self, event: &DomainEvent) -> anyhow::Result<()>;
}

pub trait EventRepository: Send + Sync {
    fn insert_pending(&self, event: &DomainEvent) -> anyhow::Result<()>;
    fn find(&self, id: i64) -> anyhow::Result<Option<ScheduledEvent>>;
    /// IDs of pending events, cheapest read for the hot poll loop — callers
    /// `find` each one right before mutating it.
    fn list_eligible(&self) -> anyhow::Result<Vec<i64>>;
    fn update(&self, event: &ScheduledEvent) -> anyhow::Result<()>;
    fn delete(&self, id: i64) -> anyhow::Result<()>;
    /// Atomically inserts `event` into the dead-letter table and deletes the
    /// original row.
    fn dead_letter(&self, event: &DeadLetteredEvent) -> anyhow::Result<()>;
}

const SELECT_COLUMNS: &str = "id, event_type, payload, retries, created_at, updated_at, last_error";

fn row_to_scheduled_event(row: &Row) -> rusqlite::Result<ScheduledEvent> {
    let created_at: String = row.get(4)?;
    let updated_at: String = row.get(5)?;
    Ok(ScheduledEvent {
        id: row.get(0)?,
        event_type: row.get(1)?,
        payload: row.get(2)?,
        retries: row.get(3)?,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
            .with_timezone(&Utc),
        last_error: row.get(6)?,
    })
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

    fn find(&self, id: i64) -> anyhow::Result<Option<ScheduledEvent>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.query_row(
            &format!("SELECT {SELECT_COLUMNS} FROM events WHERE id = ?1"),
            params![id],
            row_to_scheduled_event,
        )
        .optional()
        .context("failed to find event")
    }

    fn list_eligible(&self) -> anyhow::Result<Vec<i64>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let mut stmt = conn
            .prepare("SELECT id FROM events WHERE status = 'pending' ORDER BY id ASC")
            .context("failed to prepare list-eligible-events query")?;
        let rows = stmt
            .query_map([], |row| row.get(0))
            .context("failed to list eligible events")?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("failed to read eligible event id")
    }

    fn update(&self, event: &ScheduledEvent) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute(
            "UPDATE events SET retries = ?2, updated_at = ?3, last_error = ?4 WHERE id = ?1",
            params![
                event.id,
                event.retries,
                event.updated_at.to_rfc3339(),
                event.last_error,
            ],
        )
        .context("failed to update event")?;
        Ok(())
    }

    fn delete(&self, id: i64) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.execute("DELETE FROM events WHERE id = ?1", params![id])
            .context("failed to delete event")?;
        Ok(())
    }

    fn dead_letter(&self, event: &DeadLetteredEvent) -> anyhow::Result<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let tx = conn
            .transaction()
            .context("failed to start dead-letter transaction")?;
        tx.execute(
            "INSERT INTO domain_events_dead_letter (original_event_id, event_type, payload, retries, last_error, created_at, failed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.original_event_id,
                event.event_type,
                event.payload,
                event.retries,
                event.last_error,
                event.created_at.to_rfc3339(),
                event.failed_at.to_rfc3339(),
            ],
        )
        .context("failed to insert event into dead letter")?;
        tx.execute(
            "DELETE FROM events WHERE id = ?1",
            params![event.original_event_id],
        )
        .context("failed to delete event after moving to dead letter")?;
        tx.commit()
            .context("failed to commit dead-letter transaction")?;
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
    use crate::domain::event::EventFailureOutcome;
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

    /// Mirrors what `DomainEventsConsumer` does on a dispatch failure: read
    /// the event, let it decide retry vs. dead-letter, then persist that
    /// decision.
    fn apply_failure(repo: &SqliteEventRepository, id: i64, error: &str) {
        let scheduled = repo.find(id).unwrap().unwrap();
        match scheduled.fail(error, repo.clock.now()) {
            EventFailureOutcome::Retry(retried) => repo.update(&retried).unwrap(),
            EventFailureOutcome::DeadLetter(dead) => repo.dead_letter(&dead).unwrap(),
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
        let found = repo.find(eligible[0]).unwrap().unwrap();
        assert_eq!(found.event_type, "playlist_created");
        assert_eq!(found.payload, event().payload().to_string());
    }

    #[test]
    fn it_should_no_longer_list_an_event_as_eligible_once_deleted() {
        let repo = repo();
        repo.insert_pending(&event()).unwrap();
        let id = repo.list_eligible().unwrap()[0];

        repo.delete(id).unwrap();

        assert!(repo.list_eligible().unwrap().is_empty());
        assert!(repo.find(id).unwrap().is_none());
    }

    #[test]
    fn it_should_keep_a_failed_event_pending_and_increment_retries_below_the_limit() {
        let repo = repo();
        repo.insert_pending(&event()).unwrap();
        let id = repo.list_eligible().unwrap()[0];

        apply_failure(&repo, id, "boom");

        assert_eq!(repo.list_eligible().unwrap().len(), 1);
        assert_eq!(repo.find(id).unwrap().unwrap().retries, 1);
    }

    #[test]
    fn it_should_drop_the_event_after_the_fifth_failed_attempt() {
        let repo = repo();
        repo.insert_pending(&event()).unwrap();
        let id = repo.list_eligible().unwrap()[0];

        for _ in 0..5 {
            apply_failure(&repo, id, "boom");
        }

        assert!(repo.list_eligible().unwrap().is_empty());
    }

    #[test]
    fn it_should_move_the_event_to_the_dead_letter_table_after_the_fifth_failed_attempt() {
        let repo = repo();
        repo.insert_pending(&event()).unwrap();
        let id = repo.list_eligible().unwrap()[0];

        for _ in 0..5 {
            apply_failure(&repo, id, "boom");
        }

        assert!(repo.list_eligible().unwrap().is_empty());
        assert!(repo.find(id).unwrap().is_none());

        let conn = repo.conn.lock().unwrap();
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
        let id = repo.list_eligible().unwrap()[0];

        for _ in 0..4 {
            apply_failure(&repo, id, "boom");
        }

        assert_eq!(repo.list_eligible().unwrap().len(), 1);
    }
}
