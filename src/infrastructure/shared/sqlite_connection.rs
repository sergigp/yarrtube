use anyhow::Context;
use rusqlite::Connection;
use std::path::Path;

/// Opens a connection to the database file. In production every repository
/// shares one connection (see `InfrastructureContainer`), so writes serialize
/// through a Rust mutex rather than contending for SQLite's single write lock
/// and the process can never lock itself out. WAL mode and `busy_timeout`
/// remain a backstop for the short-lived connections opened at startup
/// (migrations, the connectivity check) and for any external tool that opens
/// the file: they let SQLite retry internally for up to 5s instead of
/// immediately returning `SQLITE_BUSY` ("database is locked"). `synchronous =
/// NORMAL` is the recommended durability level under WAL and lowers the
/// per-commit fsync cost.
pub fn open(path: &Path) -> anyhow::Result<Connection> {
    let conn =
        Connection::open(path).with_context(|| format!("failed to open database at {path:?}"))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .context("failed to enable WAL journal mode")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .context("failed to set busy timeout")?;
    conn.pragma_update(None, "synchronous", "NORMAL")
        .context("failed to set synchronous mode")?;
    Ok(conn)
}

/// A freshly migrated database file, private to one test and removed when
/// dropped. Connections are opened exactly as in production (one per
/// repository, WAL, busy timeout), so tests exercise the same multi-connection
/// setup the daemon runs with.
#[cfg(test)]
pub struct TestDatabase {
    dir: tempfile::TempDir,
}

#[cfg(test)]
impl TestDatabase {
    pub fn new() -> Self {
        let database = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        crate::infrastructure::shared::sqlite_migrations::apply(&mut database.connection())
            .unwrap();
        database
    }

    pub fn connection(&self) -> Connection {
        open(&self.path()).unwrap()
    }

    /// The database file, for code that opens its own connections.
    pub fn path(&self) -> std::path::PathBuf {
        self.dir.path().join("yarrtube.sqlite3")
    }

    pub fn shared_connection(&self) -> std::sync::Arc<std::sync::Mutex<Connection>> {
        std::sync::Arc::new(std::sync::Mutex::new(self.connection()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::Task;
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::shared::system_clock::{Clock, FixedClock};
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    #[test]
    fn it_should_share_one_migrated_database_across_its_connections() {
        let database = TestDatabase::new();
        let writer = database.connection();
        let reader = database.connection();

        writer
            .execute(
                "INSERT INTO channels (id, name, youtube_channel_id, quality, video_limit, path, created_at)
                 VALUES ('@somechannel', 'Some Channel', 'UC123', 'high', 10, 'creators/somechannel', '2024-01-01T00:00:00+00:00')",
                [],
            )
            .unwrap();

        let name: String = reader
            .query_row("SELECT name FROM channels", [], |row| row.get(0))
            .unwrap();
        assert_eq!(name, "Some Channel");
    }

    #[test]
    fn it_should_keep_wal_and_apply_normal_synchronous_on_open() {
        let database = TestDatabase::new();
        let conn = database.connection();

        let journal_mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        let synchronous: i32 = conn
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .unwrap();

        assert_eq!(journal_mode, "wal");
        assert_eq!(synchronous, 1); // 1 == NORMAL
    }

    #[test]
    fn it_should_not_fail_when_two_repositories_write_concurrently_on_the_shared_connection() {
        let database = TestDatabase::new();
        let conn = database.shared_connection();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(fixed_now()));
        let repository_a = Arc::new(SqliteTaskRepository::new(conn.clone(), clock.clone()));
        let repository_b = Arc::new(SqliteTaskRepository::new(conn.clone(), clock.clone()));

        let writes_a = spawn_writes(repository_a.clone(), "A");
        let writes_b = spawn_writes(repository_b.clone(), "B");
        let results: Vec<_> = writes_a
            .join()
            .unwrap()
            .into_iter()
            .chain(writes_b.join().unwrap())
            .collect();

        assert!(results.iter().all(Result::is_ok));
        assert_eq!(repository_a.list_non_completed().unwrap().len(), 100);
    }

    #[test]
    fn it_should_claim_a_task_while_another_repository_writes() {
        let database = TestDatabase::new();
        let conn = database.shared_connection();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(fixed_now()));
        let writer = Arc::new(SqliteTaskRepository::new(conn.clone(), clock.clone()));
        let claimer = SqliteTaskRepository::new(conn.clone(), clock.clone());
        claimer.schedule(&reconcile("target"), fixed_now()).unwrap();
        let id = claimer.list_eligible().unwrap().first().unwrap().id;

        let writes = spawn_writes(writer.clone(), "W");
        let claimed = claimer.claim(id, fixed_now());
        let write_results = writes.join().unwrap();

        assert!(claimed.unwrap().is_some());
        assert!(write_results.iter().all(Result::is_ok));
    }

    /// Schedules 50 distinct tasks from its own thread, returning each write's
    /// result so the caller can assert none hit a database-locked error.
    fn spawn_writes(
        repository: Arc<SqliteTaskRepository>,
        prefix: &str,
    ) -> std::thread::JoinHandle<Vec<anyhow::Result<()>>> {
        let prefix = prefix.to_string();
        std::thread::spawn(move || {
            (0..50)
                .map(|i| repository.schedule(&reconcile(&format!("{prefix}{i}")), fixed_now()))
                .collect()
        })
    }

    fn reconcile(playlist_id: &str) -> Task {
        Task::ReconcilePlaylist {
            playlist_id: playlist_id.to_string(),
        }
    }

    fn fixed_now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }
}
