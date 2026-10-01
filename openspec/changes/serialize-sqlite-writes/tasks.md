## 1. Walking skeleton

- [x] 1.1 Change the six plain repositories (`sqlite_playlist_repository`, `sqlite_channel_repository`, `sqlite_video_repository`, `sqlite_playlist_video_repository`, `sqlite_channel_video_repository`, `sqlite_video_metadata_repository`) to store `conn: Arc<Mutex<Connection>>` and take `new(conn: Arc<Mutex<Connection>>)`; in `infrastructure_container.rs` open the connection once via `open_shared` and pass `conn.clone()` to every repository; add `PRAGMA synchronous = NORMAL` to `sqlite_connection::open`; update every affected test/caller to construct repositories from one shared connection (`TestDatabase::shared_connection`, or `Arc::new(Mutex::new(..))` for in-memory cases). Done when `cargo build` succeeds and all existing tests pass.

## 2. Behaviour (TDD)

- [ ] 2.1 No application-layer behaviour tests apply — this change alters the infrastructure connection model only, with no new HTTP handler, subscriber, or task behaviour. (Section intentionally empty.)

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 `it_should_keep_wal_and_apply_normal_synchronous_on_open` — open a connection via `sqlite_connection::open` and assert `PRAGMA journal_mode` is `wal` and `PRAGMA synchronous` is `1` (NORMAL).
- [ ] 3.2 `it_should_not_fail_when_two_repositories_write_concurrently_on_the_shared_connection` — build two repositories from one `shared_connection`, issue concurrent writes from both, assert every write succeeds (no database-locked error) and both rows are present.
- [ ] 3.3 `it_should_claim_a_task_while_another_repository_writes` — on one shared connection, drive a burst of writes from one repository while the task repository claims an eligible task; assert the claim returns the task rather than a database-locked error.

## 4. Verification

- [ ] 4.1 Run `cargo test --locked`, `cargo fmt --all -- --check`, and `cargo clippy --all-targets --all-features --locked -- -D warnings`; all pass.
- [ ] 4.2 Manually verify against the live scenario: run `scripts/run-local.sh`, track a large playlist, and confirm no `database is locked` errors appear in the logs while downloads and task claims proceed.
