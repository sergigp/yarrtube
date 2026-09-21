## 1. Migration runner

- [x] 1.1 Add `rusqlite_migration` to `Cargo.toml` and verify `cargo build` succeeds
- [x] 1.2 Create `migrations/0001_baseline.sql` with the 8 existing `CREATE TABLE IF NOT EXISTS` statements copied verbatim from the repositories
- [x] 1.3 Add `src/infrastructure/shared/sqlite_migrations.rs` with `apply(conn: &mut Connection) -> anyhow::Result<()>` wrapping `rusqlite_migration::Migrations` over `0001_baseline.sql`, exported from `src/infrastructure/shared/mod.rs`
- [x] 1.4 Write `it_should_create_every_table_on_a_fresh_in_memory_database`, `it_should_be_a_no_op_when_applied_twice`, and `it_should_recognize_an_already_deployed_database_as_up_to_date_without_touching_its_data` in `sqlite_migrations.rs` and verify they pass

## 2. Wire migrations into startup

- [x] 2.1 Add `run_startup_migrations()` to `src/serve.rs`, calling `open_connection()` then `sqlite_migrations::apply()`
- [x] 2.2 Call `run_startup_migrations()` in `serve::run()` before `build_application()`, returning `ExitCode::FAILURE` on error (same pattern as `build_application()`'s own error handling), and verify with `cargo build`

## 3. Remove schema creation from repositories

- [x] 3.1 Remove the `CREATE TABLE` call and the now-unneeded `anyhow::Result` from `SqliteChannelRepository::new()`, update its call site in `src/serve.rs`, and verify `cargo test sqlite_channel_repository` passes
- [x] 3.2 Do the same for `SqlitePlaylistRepository::new()` and verify `cargo test sqlite_playlist_repository` passes
- [x] 3.3 Do the same for `SqliteVideoRepository::new()` and verify `cargo test sqlite_video_repository` passes
- [x] 3.4 Do the same for `SqliteVideoMetadataRepository::new()` and verify `cargo test sqlite_video_metadata_repository` passes
- [x] 3.5 Do the same for `SqlitePlaylistVideoRepository::new()`, update its test `repo()` helper to call `sqlite_migrations::apply()` instead of hand-rolling a partial `videos` table, and verify `cargo test sqlite_playlist_video_repository` passes
- [x] 3.6 Do the same for `SqliteChannelVideoRepository::new()`, update its test `repo()` helper to call `sqlite_migrations::apply()` instead of hand-rolling a partial `videos` table, and verify `cargo test sqlite_channel_video_repository` passes
- [x] 3.7 Remove both `CREATE TABLE` calls and the lock-only fallibility from `SqliteTaskRepository::new()`, update its call site in `src/serve.rs`, and verify `cargo test sqlite_task_repository` passes
- [x] 3.8 Delete `create_events_table` and `create_domain_events_dead_letter_table` from `event_repository.rs`, remove their calls and the now-unneeded `anyhow::Result` from `SqliteEventRepository::new()`, update its call site in `src/serve.rs`, and verify `cargo test event_repository` passes
- [x] 3.9 Remove the `create_events_table` call and the now-unneeded `anyhow::Result` from `SqliteEventPublisher::new()`, update its call site in `src/serve.rs`, and verify `cargo test event_publisher` passes

## 4. Full verification

- [x] 4.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`, and confirm all pass
- [x] 4.2 Run `scripts/run-local.sh` against a fresh `yarrtube.sqlite3`, confirm all tables are created and the app serves normally
- [x] 4.3 Run `scripts/run-local.sh` against a copy of a pre-migration `yarrtube.sqlite3` (schema created by the old per-repository `CREATE TABLE IF NOT EXISTS` calls, with existing rows), confirm startup succeeds, no data is lost, and the app serves normally
