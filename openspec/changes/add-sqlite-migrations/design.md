## Files

- `Cargo.toml` — add `rusqlite_migration` dependency.
- `migrations/0001_baseline.sql` — new; the 8 existing `CREATE TABLE IF NOT EXISTS` statements, copied verbatim, so a deployed database is recognized at version 1 with zero drift.
- `src/infrastructure/shared/sqlite_migrations.rs` — new; wraps `rusqlite_migration::Migrations`, single place that knows the migration file list.
- `src/infrastructure/shared/mod.rs` — export the new module.
- `src/serve.rs` — runs migrations once, before `build_application()` opens the per-repository connections.
- `src/infrastructure/repositories/sqlite_channel_repository.rs`, `sqlite_playlist_repository.rs`, `sqlite_playlist_video_repository.rs`, `sqlite_video_repository.rs`, `sqlite_video_metadata_repository.rs`, `sqlite_channel_video_repository.rs` — drop the `CREATE TABLE` call from `new()`; it was their only fallible step, so `new()` stops returning `anyhow::Result`.
- `src/infrastructure/repositories/sqlite_task_repository.rs` — drop both `CREATE TABLE` calls (and the lock they required) from `new()`; same infallibility simplification.
- `src/infrastructure/shared/domain_events/event_repository.rs` — delete `create_events_table` and `create_domain_events_dead_letter_table`; drop the `CREATE TABLE` calls from `SqliteEventRepository::new()`.
- `src/infrastructure/shared/domain_events/event_publisher.rs` — drop the `create_events_table` call from `SqliteEventPublisher::new()`.
- `sqlite_playlist_video_repository.rs` / `sqlite_channel_video_repository.rs` test modules — their `repo()` test helper applies real migrations instead of hand-rolling a partial `videos` table.
- `src/serve.rs` (`build_application`, tests) — updated call sites for every constructor whose signature changed.

## Types & Signatures

```rust
// src/infrastructure/shared/sqlite_migrations.rs
pub fn apply(conn: &mut rusqlite::Connection) -> anyhow::Result<()>;

// src/serve.rs
fn run_startup_migrations() -> anyhow::Result<()>;

// constructors that only failed via CREATE TABLE become infallible
impl SqliteChannelRepository { pub fn new(conn: Connection) -> Self }
impl SqlitePlaylistRepository { pub fn new(conn: Connection) -> Self }
impl SqlitePlaylistVideoRepository { pub fn new(conn: Connection) -> Self }
impl SqliteVideoRepository { pub fn new(conn: Connection) -> Self }
impl SqliteVideoMetadataRepository { pub fn new(conn: Connection) -> Self }
impl SqliteChannelVideoRepository { pub fn new(conn: Connection) -> Self }
impl SqliteTaskRepository { pub fn new(conn: Arc<Mutex<Connection>>, clock: Arc<dyn Clock>) -> Self }
impl SqliteEventRepository { pub fn new(conn: Arc<Mutex<Connection>>) -> Self }
impl SqliteEventPublisher { pub fn new(conn: Arc<Mutex<Connection>>, clock: Arc<dyn Clock>) -> Self }
```

## Call Stack

```
main() -> serve::run()
  run_startup_migrations()                          // src/serve.rs
    open_connection()                                // existing helper, unchanged
    sqlite_migrations::apply(&mut conn)               // src/infrastructure/shared/sqlite_migrations.rs
      Migrations::new(vec![M::up(BASELINE_SQL)])
        .to_latest(conn)
  // ExitCode::FAILURE if run_startup_migrations() errs, same handling as build_application()
  build_application()
    open_connection() x7                              // unchanged; repos no longer touch schema
    SqliteChannelRepository::new(conn)                 // no longer fallible
    ...
```

## Test Plan

- `sqlite_migrations::tests::it_should_create_every_table_on_a_fresh_in_memory_database` — applies migrations to `Connection::open_in_memory()`, asserts all 10 tables exist via `sqlite_master`.
- `sqlite_migrations::tests::it_should_be_a_no_op_when_applied_twice` — calls `apply()` twice on the same connection, asserts the second call succeeds and the schema is unchanged.
- `sqlite_migrations::tests::it_should_recognize_an_already_deployed_database_as_up_to_date_without_touching_its_data` — creates the 8 tables using today's raw `CREATE TABLE IF NOT EXISTS` statements (simulating a pre-migration production DB), inserts a row, then applies migrations and asserts the row is untouched and the schema version is latest.
- `sqlite_playlist_video_repository::tests` — `repo()` helper switched to `sqlite_migrations::apply()`; existing tests must keep passing against the full production `videos` schema instead of the old partial one.
- `sqlite_channel_video_repository::tests` — same helper switch, existing tests must keep passing.
- `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass.
