## 1. Walking skeleton

- [x] 1.1 Change the six plain repositories (`sqlite_playlist_repository`, `sqlite_channel_repository`, `sqlite_video_repository`, `sqlite_playlist_video_repository`, `sqlite_channel_video_repository`, `sqlite_video_metadata_repository`) to store `conn: Arc<Mutex<Connection>>` and take `new(conn: Arc<Mutex<Connection>>)`; in `infrastructure_container.rs` open the connection once via `open_shared` and pass `conn.clone()` to every repository; add `PRAGMA synchronous = NORMAL` to `sqlite_connection::open`; update every affected test/caller to construct repositories from one shared connection (`TestDatabase::shared_connection`, or `Arc::new(Mutex::new(..))` for in-memory cases). Done when `cargo build` succeeds and all existing tests pass.

- [x] 1.2 Introduce a `Database` handle (one read connection + one write connection, each behind a mutex) in `sqlite_connection.rs` with `read()` / `write()` guard accessors and `open`; change every repository to hold a `Database` and take it in `new`, routing pure reads to `self.db.read()?` and writes/claim/transactions to `self.db.write()?`; in `infrastructure_container.rs` open one `Database` and inject `db.clone()` into every repository (superseding 1.1's single shared connection); update `TestDatabase` and every repository construction in tests to build from a `Database`. Done when `cargo build` succeeds and all existing tests pass.

## 2. Behaviour (TDD)

- [x] 2.1 No application-layer behaviour tests apply — this change alters the infrastructure connection model only, with no new HTTP handler, subscriber, or task behaviour. (Section intentionally empty.)

## 3. Infrastructure adapters (TDD)

- [x] 3.1 `it_should_keep_wal_and_apply_normal_synchronous_on_open` — open a connection via `sqlite_connection::open` and assert `PRAGMA journal_mode` is `wal` and `PRAGMA synchronous` is `1` (NORMAL).
- [x] 3.2 `it_should_not_fail_when_two_repositories_write_concurrently_on_the_shared_connection` — build two repositories from one `shared_connection`, issue concurrent writes from both, assert every write succeeds (no database-locked error) and both rows are present.
- [x] 3.3 `it_should_claim_a_task_while_another_repository_writes` — on one shared connection, drive a burst of writes from one repository while the task repository claims an eligible task; assert the claim returns the task rather than a database-locked error.
- [x] 3.4 `it_should_read_while_the_write_connection_is_held` — seed a row, hold the `Database` write guard open on one thread, then read through the read connection; assert the read returns the seeded row without waiting for the write lock (a hang would mean reads share the write mutex).

## 4. Verification

- [x] 4.1 Run `cargo test --locked`, `cargo fmt --all -- --check`, and `cargo clippy --all-targets --all-features --locked -- -D warnings`; all pass (write-path).
- [x] 4.2 Re-run the full gate (`cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`) after the read-connection split; all pass.
- [ ] 4.3 Manually verify against the live scenario: run `scripts/run-local.sh`, track a large playlist, and confirm no `database is locked` errors appear in the logs and the web UI stays responsive (reads not blocked) while downloads and task claims proceed.
