use anyhow::Context;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Opens a connection to the database file. In production the process shares a
/// single `Database` (a write connection and a read connection; see
/// `InfrastructureContainer`), so writes serialize through a Rust mutex rather
/// than contending for SQLite's single write lock and the process can never
/// lock itself out. WAL mode and `busy_timeout` remain a backstop for the
/// short-lived connections opened at startup (migrations, the connectivity
/// check) and for any external tool that opens the file: they let SQLite retry
/// internally for up to 5s instead of immediately returning `SQLITE_BUSY`
/// ("database is locked"). `synchronous = NORMAL` is the recommended durability
/// level under WAL and lowers the per-commit fsync cost.
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

/// The process's database access, injected into every repository. It holds two
/// connections: a single `write` connection, through which every write
/// serializes (one writer, so the process can never lock itself out), and a
/// `read` connection. Because the two are distinct connections, WAL lets a read
/// run concurrently with an in-progress write, so a write burst (e.g. ingesting
/// a large playlist) does not block the web UI's reads.
#[derive(Clone)]
pub struct Database {
    // Deliberately a *single* read connection, not a pool. A pool would let
    // reads run concurrently with *each other* too, but that is YAGNI here:
    // this is a single-user web UI, so there is effectively never more than one
    // read in flight. The one read connection already buys the property we need
    // — reads are not blocked by the writer (WAL reader/writer concurrency) —
    // and reads serializing among themselves behind this mutex is unobservable
    // because each is a sub-millisecond `SELECT`. If the access pattern ever
    // becomes genuinely read-concurrent, grow this into a small pool; the write
    // path does not change.
    read: Arc<Mutex<Connection>>,
    write: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            read: Arc::new(Mutex::new(open(path)?)),
            write: Arc::new(Mutex::new(open(path)?)),
        })
    }

    /// The read connection, for pure `SELECT`s.
    pub fn read(&self) -> &Mutex<Connection> {
        &self.read
    }

    /// The write connection, for every `INSERT`/`UPDATE`/`DELETE`, the
    /// read-modify-write task claim, and transactions.
    pub fn write(&self) -> &Mutex<Connection> {
        &self.write
    }

    /// A `Database` backed by one already-open connection used for both roles.
    /// Only for tests whose connection cannot be reopened by path: an in-memory
    /// database is private to its connection, so two connections would be two
    /// different databases.
    #[cfg(test)]
    pub fn single(conn: Connection) -> Self {
        let shared = Arc::new(Mutex::new(conn));
        Self {
            read: shared.clone(),
            write: shared,
        }
    }
}

/// A freshly migrated database file, private to one test and removed when
/// dropped. Connections are opened exactly as in production (via `open`: WAL,
/// busy timeout, `synchronous = NORMAL`), so tests exercise the same setup the
/// daemon runs with.
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

    /// A `Database` over this test's file, with its own read and write
    /// connections — the same two-connection setup production uses.
    pub fn database(&self) -> Database {
        Database::open(&self.path()).unwrap()
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
    fn it_should_not_fail_when_two_repositories_write_concurrently_on_the_shared_database() {
        let database = TestDatabase::new();
        let db = database.database();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(fixed_now()));
        let repository_a = Arc::new(SqliteTaskRepository::new(db.clone(), clock.clone()));
        let repository_b = Arc::new(SqliteTaskRepository::new(db.clone(), clock.clone()));

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
        let db = database.database();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(fixed_now()));
        let writer = Arc::new(SqliteTaskRepository::new(db.clone(), clock.clone()));
        let claimer = SqliteTaskRepository::new(db.clone(), clock.clone());
        claimer.schedule(&reconcile("target"), fixed_now()).unwrap();
        let id = claimer.list_eligible().unwrap().first().unwrap().id;

        let writes = spawn_writes(writer.clone(), "W");
        let claimed = claimer.claim(id, fixed_now());
        let write_results = writes.join().unwrap();

        assert!(claimed.unwrap().is_some());
        assert!(write_results.iter().all(Result::is_ok));
    }

    #[test]
    fn it_should_read_while_the_write_connection_is_held() {
        let database = TestDatabase::new();
        let db = database.database();
        db.write()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO channels (id, name, youtube_channel_id, quality, video_limit, path, created_at)
                 VALUES ('@c', 'Held', 'UC1', 'high', 10, 'creators/c', '2024-01-01T00:00:00+00:00')",
                [],
            )
            .unwrap();

        // Hold the write connection open, as an in-progress write would, then
        // read: with a separate read connection this must not wait on the write
        // lock (sharing one connection would deadlock here instead).
        let _write_guard = db.write().lock().unwrap();
        let name: String = db
            .read()
            .lock()
            .unwrap()
            .query_row("SELECT name FROM channels", [], |row| row.get(0))
            .unwrap();

        assert_eq!(name, "Held");
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
