_Decision: use one shared connection, not a connection pool. A pool (sqlx/r2d2/deadpool) scales reads, but SQLite allows one writer regardless of connection count, so more connections means more write contention, not less. One shared connection is the simplest form of the standard "single writer (+ optional read pool)" SQLite pattern; reads here are small and infrequent, so a read pool is not needed._

## Files

- `src/infrastructure/shared/sqlite_connection.rs` — add `PRAGMA synchronous = NORMAL` in `open`; WAL and `busy_timeout` stay (backstop for startup/external connections).
- `src/infrastructure/infrastructure_container.rs` — open the connection once and share one `Arc<Mutex<Connection>>` across every repository.
- `src/infrastructure/repositories/sqlite_playlist_repository.rs`, `sqlite_channel_repository.rs`, `sqlite_video_repository.rs`, `sqlite_playlist_video_repository.rs`, `sqlite_channel_video_repository.rs`, `sqlite_video_metadata_repository.rs` — change the stored field and constructor from an owned `Connection` to the shared `Arc<Mutex<Connection>>` (task/event repositories already take this). Method bodies are unchanged: `self.conn.lock()` works the same through the `Arc`.
- Tests across those repositories and their application-layer callers — construct repositories from one shared connection instead of a connection each (`TestDatabase::shared_connection`, or `Arc::new(Mutex::new(..))` for the in-memory cases).

## Types & Signatures

```rust
// sqlite_connection.rs — one added pragma
conn.pragma_update(None, "synchronous", "NORMAL")?;

// each of the six plain repositories
struct Sqlite<Name>Repository {
    conn: Arc<Mutex<Connection>>, // was: Mutex<Connection>
}
impl Sqlite<Name>Repository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self { Self { conn } } // was: new(conn: Connection)
}

// infrastructure_container.rs
let conn = open_shared(db_path)?;                 // Arc<Mutex<Connection>>, opened once
// every repository receives conn.clone()
```

## Call Stack

- `InfrastructureContainer::new(settings)` → `open_shared(db_path)` once → `conn.clone()` passed to each `Sqlite*Repository::new(..)`. All repositories now share one `Arc<Mutex<Connection>>`.
- Any repository method, e.g. `SqliteTaskRepository::claim(id, now)` / `SqliteVideoRepository::update(video)` → `self.conn.lock()` → serialized access to the one connection → single SQL statement → guard dropped. No two database operations in the process run against SQLite at once.

## Test Plan

Infrastructure tests (one group — the shared-connection adapter):

1. `it_should_not_fail_when_two_repositories_write_concurrently_on_the_shared_connection` — build two repositories from one `shared_connection`, spawn concurrent writes from both, assert every write succeeds (no database-locked error) and both rows are present.
2. `it_should_claim_a_task_while_another_repository_writes` — on the shared connection, drive a burst of writes from one repository while claiming an eligible task from the task repository; assert the claim returns the task rather than a lock error.
3. `it_should_keep_wal_and_apply_normal_synchronous_on_open` — open a connection via `sqlite_connection::open` and assert `PRAGMA journal_mode` is `wal` and `PRAGMA synchronous` is `1` (NORMAL).

Existing repository and application-layer tests are updated to build repositories from a single shared connection; their assertions are unchanged.
