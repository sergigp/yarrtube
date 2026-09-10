use crate::domain::event::DomainEvent;
use crate::infrastructure::shared::domain_events::event_repository::create_events_table;
use crate::infrastructure::shared::system_clock::Clock;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};
use tracing::info;

pub trait EventPublisher: Send + Sync {
    fn publish(&self, event: &DomainEvent) -> anyhow::Result<()>;
}

fn insert_pending_row(
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

pub struct SqliteEventPublisher {
    conn: Arc<Mutex<Connection>>,
    clock: Arc<dyn Clock>,
}

impl SqliteEventPublisher {
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

impl EventPublisher for SqliteEventPublisher {
    fn publish(&self, event: &DomainEvent) -> anyhow::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        insert_pending_row(&conn, event, self.clock.now())
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
