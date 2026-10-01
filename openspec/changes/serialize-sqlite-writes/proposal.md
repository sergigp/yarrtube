## Why

Adding a large playlist produces a burst of concurrent database writes that fail with `database is locked` (SQLite `SQLITE_BUSY`). The single `serve` process opens nine independent SQLite connections — one per repository — and SQLite permits only one writer at a time even in WAL mode, so the burst saturates the write lock. `busy_timeout(5s)` offers no fairness guarantee, so an unlucky connection (observed in production: the task executor's `claim`) starves for the full timeout and then errors, stalling task processing.

## What Changes

- Route every repository through a single shared database connection so all in-process database access serializes through a Rust mutex instead of contending for SQLite's write lock. With one writer, the process can no longer lock itself out.
- Set `PRAGMA synchronous = NORMAL` (safe and recommended under WAL) to lower per-write fsync cost now that writes are serialized.
- Keep the existing `busy_timeout` as a backstop for the brief startup migration/connectivity connections and for any external tooling that opens the file.
- No change to the HTTP API, task semantics, or on-disk schema.

## Capabilities

### New Capabilities
<!-- None -->

### Modified Capabilities
- `daemon`: Adds a requirement that the daemon serializes its database access so that concurrent writes under load complete rather than failing with database-locked errors.

## Impact

- `src/infrastructure/infrastructure_container.rs`: wire all repositories to one shared `Arc<Mutex<Connection>>` instead of opening a connection each.
- Repository constructors that currently take an owned `Connection` (`SqlitePlaylistRepository`, `SqliteChannelRepository`, `SqliteVideoRepository`, `SqlitePlaylistVideoRepository`, `SqliteChannelVideoRepository`, `SqliteVideoMetadataRepository`): change to accept the shared `Arc<Mutex<Connection>>`, matching the task/event repositories that already do.
- `src/infrastructure/shared/sqlite_connection.rs`: add `PRAGMA synchronous = NORMAL`; `busy_timeout` and WAL stay.
- No new dependencies. A connection pool (sqlx / r2d2 / deadpool) was considered and rejected — see design.md.
