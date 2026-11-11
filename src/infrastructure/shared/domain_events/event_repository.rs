#[cfg(test)]
use crate::domain::event::DomainEvent;
use crate::domain::event::{DeadLetteredEvent, ScheduledEvent};
use crate::infrastructure::shared::sqlite_connection::Database;
use crate::infrastructure::shared::system_clock::Clock;
use anyhow::Context;
use chrono::{DateTime, Utc};
#[cfg(test)]
use rusqlite::Connection;
#[cfg(test)]
use rusqlite::OptionalExtension;
use rusqlite::{Row, params};
use std::sync::Arc;

pub trait EventRepository: Send + Sync {
    /// Pending events whose `run_at` has passed, in publication (id) order.
    fn list_eligible(&self) -> anyhow::Result<Vec<ScheduledEvent>>;
    /// Every pending event, backing off or not, in publication (id) order.
    /// What anything other than dispatch means by "not handled yet".
    fn list_pending(&self) -> anyhow::Result<Vec<ScheduledEvent>>;
    fn update(&self, event: &ScheduledEvent) -> anyhow::Result<()>;
    fn delete(&self, id: i64) -> anyhow::Result<()>;
    /// Atomically inserts `event` into the dead-letter table and deletes the
    /// original row.
    fn dead_letter(&self, event: &DeadLetteredEvent) -> anyhow::Result<()>;
}

const SELECT_COLUMNS: &str =
    "id, event_type, payload, retries, created_at, updated_at, last_error, run_at";

fn row_to_scheduled_event(row: &Row) -> rusqlite::Result<ScheduledEvent> {
    Ok(ScheduledEvent {
        id: row.get(0)?,
        event_type: row.get(1)?,
        payload: row.get(2)?,
        retries: row.get(3)?,
        run_at: timestamp(row, 7)?,
        created_at: timestamp(row, 4)?,
        updated_at: timestamp(row, 5)?,
        last_error: row.get(6)?,
    })
}

/// The RFC 3339 timestamp stored in column `idx`.
fn timestamp(row: &Row, idx: usize) -> rusqlite::Result<DateTime<Utc>> {
    let value: String = row.get(idx)?;
    DateTime::parse_from_rfc3339(&value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, Box::new(e))
        })
}

pub struct SqliteEventRepository {
    db: Database,
    clock: Arc<dyn Clock>,
}

impl SqliteEventRepository {
    pub fn new(db: Database, clock: Arc<dyn Clock>) -> Self {
        Self { db, clock }
    }
}

impl SqliteEventRepository {
    #[cfg(test)]
    fn find(&self, id: i64) -> anyhow::Result<Option<ScheduledEvent>> {
        let conn = self.db.read()?;
        conn.query_row(
            &format!("SELECT {SELECT_COLUMNS} FROM events WHERE id = ?1"),
            params![id],
            row_to_scheduled_event,
        )
        .optional()
        .context("failed to find event")
    }

    /// Every row in the dead-letter table, which `EventRepository` cannot read.
    #[cfg(test)]
    pub fn list_dead_lettered(&self) -> anyhow::Result<Vec<DeadLetteredEvent>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(
                "SELECT original_event_id, event_type, payload, retries, last_error, created_at, failed_at
                 FROM domain_events_dead_letter ORDER BY id ASC",
            )
            .context("failed to prepare dead-lettered events query")?;
        let rows = stmt
            .query_map([], |row| {
                let created_at: String = row.get(5)?;
                let failed_at: String = row.get(6)?;
                Ok(DeadLetteredEvent {
                    original_event_id: row.get(0)?,
                    event_type: row.get(1)?,
                    payload: row.get(2)?,
                    retries: row.get(3)?,
                    last_error: row.get(4)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .unwrap()
                        .with_timezone(&Utc),
                    failed_at: DateTime::parse_from_rfc3339(&failed_at)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })
            .context("failed to list dead-lettered events")?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("failed to read dead-lettered event row")
    }
}

impl EventRepository for SqliteEventRepository {
    fn list_eligible(&self) -> anyhow::Result<Vec<ScheduledEvent>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {SELECT_COLUMNS} FROM events WHERE status = 'pending' AND run_at <= ?1 ORDER BY id ASC"
            ))
            .context("failed to prepare list-eligible-events query")?;
        let rows = stmt
            .query_map(
                params![self.clock.now().to_rfc3339()],
                row_to_scheduled_event,
            )
            .context("failed to list eligible events")?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("failed to read eligible event row")
    }

    fn list_pending(&self) -> anyhow::Result<Vec<ScheduledEvent>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {SELECT_COLUMNS} FROM events WHERE status = 'pending' ORDER BY id ASC"
            ))
            .context("failed to prepare list-pending-events query")?;
        let rows = stmt
            .query_map([], row_to_scheduled_event)
            .context("failed to list pending events")?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("failed to read pending event row")
    }

    fn update(&self, event: &ScheduledEvent) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "UPDATE events SET retries = ?2, updated_at = ?3, last_error = ?4, run_at = ?5 WHERE id = ?1",
            params![
                event.id,
                event.retries,
                event.updated_at.to_rfc3339(),
                event.last_error,
                event.run_at.to_rfc3339(),
            ],
        )
        .context("failed to update event")?;
        Ok(())
    }

    fn delete(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute("DELETE FROM events WHERE id = ?1", params![id])
            .context("failed to delete event")?;
        Ok(())
    }

    fn dead_letter(&self, event: &DeadLetteredEvent) -> anyhow::Result<()> {
        let mut conn = self.db.write()?;
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
mod tests {
    use super::*;
    use crate::domain::event::EventFailureOutcome;
    use crate::domain::playlist::PlaylistCreated;
    use crate::infrastructure::shared::domain_events::event_publisher::{
        EventPublisher, SqliteEventPublisher,
    };
    use crate::infrastructure::shared::system_clock::FixedClock;

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(0, 0).unwrap()
    }

    /// Publishes through `SqliteEventPublisher` (sharing the repository's
    /// connection) since `EventRepository` itself no longer writes new rows.
    fn repo_with_one_pending_event() -> (SqliteEventRepository, i64) {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::shared::sqlite_migrations::apply(&mut conn).unwrap();
        let conn = Database::single(conn);
        let publisher = SqliteEventPublisher::new(conn.clone(), Arc::new(FixedClock(now())));
        let repo = SqliteEventRepository::new(conn, Arc::new(FixedClock(now())));
        publisher.publish(&event()).unwrap();
        let id = repo.list_eligible().unwrap()[0].id;
        (repo, id)
    }

    fn repo() -> SqliteEventRepository {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::shared::sqlite_migrations::apply(&mut conn).unwrap();
        SqliteEventRepository::new(Database::single(conn), Arc::new(FixedClock(now())))
    }

    fn event() -> DomainEvent {
        DomainEvent::PlaylistCreated(PlaylistCreated {
            playlist_id: "PL1".to_string(),
        })
    }

    /// Mirrors what `DomainEventsConsumer` does on a dispatch failure: read
    /// the event, let it decide retry vs. dead-letter, then persist that
    /// decision. A zero base delay keeps a retried event due immediately:
    /// these tests are about the retry counter, not the delay.
    fn apply_failure(repo: &SqliteEventRepository, id: i64, error: &str) {
        let scheduled = repo.find(id).unwrap().unwrap();
        match scheduled.fail(error, now(), 0) {
            EventFailureOutcome::Retry(retried) => repo.update(&retried).unwrap(),
            EventFailureOutcome::DeadLetter(dead) => repo.dead_letter(&dead).unwrap(),
        }
    }

    #[test]
    fn it_should_create_a_queryable_empty_dead_letter_table() {
        let repo = repo();

        let conn = repo.db.read().unwrap();
        let mut stmt = conn
            .prepare("SELECT * FROM domain_events_dead_letter")
            .unwrap();
        let rows = stmt.query_map([], |_| Ok(())).unwrap();

        assert_eq!(rows.count(), 0);
    }

    #[test]
    fn it_should_list_a_newly_published_event_as_eligible() {
        let (repo, id) = repo_with_one_pending_event();

        let eligible = repo.list_eligible().unwrap();

        assert_eq!(
            eligible,
            vec![ScheduledEvent {
                id,
                event_type: "playlist_created".to_string(),
                payload: event().payload().to_string(),
                retries: 0,
                run_at: now(),
                created_at: now(),
                updated_at: now(),
                last_error: None,
            }]
        );
    }

    #[test]
    fn it_should_not_list_an_event_whose_run_at_is_in_the_future() {
        let (repo, id) = repo_with_one_pending_event();
        let one_second_later = now() + chrono::Duration::seconds(1);
        let scheduled = repo.find(id).unwrap().unwrap();
        let backing_off = ScheduledEvent {
            run_at: one_second_later,
            ..scheduled
        };
        repo.update(&backing_off).unwrap();

        let eligible_now = repo.list_eligible().unwrap();
        let eligible_later =
            SqliteEventRepository::new(repo.db.clone(), Arc::new(FixedClock(one_second_later)))
                .list_eligible()
                .unwrap();

        assert_eq!(eligible_now, Vec::<ScheduledEvent>::new());
        assert_eq!(eligible_later, vec![backing_off]);
    }

    #[test]
    fn it_should_list_a_backing_off_event_as_pending() {
        let (repo, id) = repo_with_one_pending_event();
        let scheduled = repo.find(id).unwrap().unwrap();
        let backing_off = ScheduledEvent {
            run_at: now() + chrono::Duration::seconds(1),
            ..scheduled
        };
        repo.update(&backing_off).unwrap();

        let eligible = repo.list_eligible().unwrap();
        let pending = repo.list_pending().unwrap();

        assert_eq!(eligible, Vec::<ScheduledEvent>::new());
        assert_eq!(pending, vec![backing_off]);
    }

    #[test]
    fn it_should_no_longer_list_an_event_as_eligible_once_deleted() {
        let (repo, id) = repo_with_one_pending_event();

        repo.delete(id).unwrap();

        assert!(repo.list_eligible().unwrap().is_empty());
        assert!(repo.find(id).unwrap().is_none());
    }

    #[test]
    fn it_should_keep_a_failed_event_pending_and_increment_retries_below_the_limit() {
        let (repo, id) = repo_with_one_pending_event();

        apply_failure(&repo, id, "boom");

        assert_eq!(repo.list_eligible().unwrap().len(), 1);
        assert_eq!(repo.find(id).unwrap().unwrap().retries, 1);
    }

    #[test]
    fn it_should_drop_the_event_after_the_fifth_failed_attempt() {
        let (repo, id) = repo_with_one_pending_event();

        for _ in 0..5 {
            apply_failure(&repo, id, "boom");
        }

        assert!(repo.list_eligible().unwrap().is_empty());
    }

    #[test]
    fn it_should_move_the_event_to_the_dead_letter_table_after_the_fifth_failed_attempt() {
        let (repo, id) = repo_with_one_pending_event();

        for _ in 0..5 {
            apply_failure(&repo, id, "boom");
        }

        assert!(repo.list_eligible().unwrap().is_empty());
        assert!(repo.find(id).unwrap().is_none());

        let conn = repo.db.read().unwrap();
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
        let (repo, id) = repo_with_one_pending_event();

        for _ in 0..4 {
            apply_failure(&repo, id, "boom");
        }

        assert_eq!(repo.list_eligible().unwrap().len(), 1);
    }
}
