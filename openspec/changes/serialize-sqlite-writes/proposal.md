## Why

Adding a large playlist produces a burst of concurrent database writes that fail with `database is locked` (SQLite `SQLITE_BUSY`). The single `serve` process opens nine independent SQLite connections — one per repository — and SQLite permits only one writer at a time even in WAL mode, so the burst saturates the write lock. `busy_timeout(5s)` offers no fairness guarantee, so an unlucky connection (observed in production: the task executor's `claim`) starves for the full timeout and then errors, stalling task processing.

## What Changes

- Give the process two connections — one **write** connection and one **read** connection — wrapped in a small `Database` handle injected into every repository. Writes (and read-modify-write operations such as task claim, plus transactions) go through the write connection, serialized by a mutex so the process keeps exactly one writer and can never lock itself out. Pure reads go through the read connection.
- Because the two connections are distinct, WAL lets a read proceed concurrently with an in-progress write, so a write burst (e.g. ingesting a large playlist) does not stall the web UI's reads.
- Set `PRAGMA synchronous = NORMAL` (safe and recommended under WAL) to lower per-write fsync cost.
- Keep the existing `busy_timeout` as a backstop for the brief startup migration/connectivity connections and for any external tooling that opens the file.
- No change to the HTTP API, task semantics, or on-disk schema.

## Capabilities

### New Capabilities
<!-- None -->

### Modified Capabilities
- `daemon`: Adds a requirement that the daemon serializes its writes through a single writer so concurrent operations complete rather than failing with database-locked errors, and that reads run on a separate connection so a write burst does not block them.

## Impact

- `src/infrastructure/shared/sqlite_connection.rs`: add a `Database` handle holding one read and one write `Arc<Mutex<Connection>>`, with `read()` / `write()` guard accessors and an `open`; `open` keeps WAL + `busy_timeout` and adds `PRAGMA synchronous = NORMAL`.
- `src/infrastructure/infrastructure_container.rs`: open one `Database` and inject a clone into every repository instead of opening a connection each.
- All nine repository constructors: hold a `Database` instead of an owned `Connection` / `Arc<Mutex<Connection>>`; route pure reads to `db.read()` and writes/claim/transactions to `db.write()`.
- No new dependencies. A connection pool (sqlx / r2d2 / deadpool) was considered and rejected — see design.md.
