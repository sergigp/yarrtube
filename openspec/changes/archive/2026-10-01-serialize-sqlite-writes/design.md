_Decision: use one write connection plus one read connection, not a connection pool. SQLite allows one writer regardless of connection count, so writes serialize no matter what; the write connection is a single mutex-guarded `Connection` so the process keeps one writer and can never lock itself out. A second, separate connection for reads is the minimum needed to exploit WAL's reader/writer concurrency, so a write burst (large-playlist ingest) does not stall web reads. A read **pool** (reader/reader concurrency) is deferred: a single read connection still serializes reads among themselves, but that is irrelevant for a single-user web UI — the pool is the generalization if that ever changes._

_Status: the write half (single write connection + `synchronous = NORMAL`) is already implemented and committed. This design adds the read connection and the `Database` handle that routes reads to it._

## Files

- `src/infrastructure/shared/sqlite_connection.rs` — `open` keeps WAL + `busy_timeout` and sets `synchronous = NORMAL` (done). Add a `Database` handle holding one read and one write `Arc<Mutex<Connection>>`, with `read()` / `write()` guard accessors and `open`; both connections are opened through the existing `open`.
- `src/infrastructure/infrastructure_container.rs` — open one `Database` and inject a clone into every repository.
- The nine repositories — hold a `Database` instead of a raw connection; read methods take `self.db.read()?`, write / claim / transaction methods take `self.db.write()?`.
- `TestDatabase` and every repository construction in tests — build repositories from a `Database` over the temp file.

## Types & Signatures

```rust
// infrastructure/shared/sqlite_connection.rs
#[derive(Clone)]
pub struct Database {
    read: Arc<Mutex<Connection>>,
    write: Arc<Mutex<Connection>>,
}
impl Database {
    pub fn open(path: &Path) -> anyhow::Result<Self>;                    // opens one read + one write connection via open()
    pub fn read(&self) -> anyhow::Result<MutexGuard<'_, Connection>>;   // pure SELECTs
    pub fn write(&self) -> anyhow::Result<MutexGuard<'_, Connection>>;  // INSERT/UPDATE, claim, transactions
    #[cfg(test)]
    pub fn single(conn: Connection) -> Self;                             // one connection as both read+write, for in-memory tests
}

// every repository
struct Sqlite<Name>Repository { db: Database }                          // was: conn: Arc<Mutex<Connection>>
impl Sqlite<Name>Repository {
    pub fn new(db: Database) -> Self { Self { db } }
}
// a read method:  let conn = self.db.read()?;  ...SELECT...
// a write method: let conn = self.db.write()?; ...INSERT/UPDATE/claim/tx...

// infrastructure_container.rs
let db = Database::open(db_path)?;   // every repository receives db.clone()
```

## Call Stack

- `InfrastructureContainer::new(settings)` → `Database::open(db_path)` (opens the read and write connections) → `db.clone()` into each `Sqlite*Repository::new(..)`.
- Read path: handler/task → e.g. `SqliteVideoRepository::list(..)` → `self.db.read()?` (read connection) → `SELECT`. Under WAL this proceeds concurrently with any in-progress write on the write connection.
- Write path: task → e.g. `SqliteVideoRepository::update(video)` → `self.db.write()?` (write connection, single writer) → `UPDATE`. Serialized across all repositories by the write mutex, so two writes never reach SQLite at once.

## Test Plan

Infrastructure tests (the `Database` handle):

1. `it_should_keep_wal_and_apply_normal_synchronous_on_open` (done) — a connection opened via `open` reports `journal_mode = wal` and `synchronous = 1` (NORMAL).
2. `it_should_not_fail_when_two_repositories_write_concurrently_on_the_shared_connection` (done) — two repositories writing concurrently through the write connection all succeed, no database-locked error.
3. `it_should_claim_a_task_while_another_repository_writes` (done) — a task claim succeeds while a write burst runs.
4. `it_should_read_while_the_write_connection_is_held` — seed a row, hold the write guard open on one thread, then read through the read connection; assert the read returns the seeded row without waiting for the write lock, proving reads use a separate connection (a deadlock/hang would mean reads share the write mutex).

Existing repository and application-layer tests build repositories from a `Database`; their assertions are unchanged.
