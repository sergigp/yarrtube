## 1. Startup seeding guard

- [x] 1.1 In `serve.rs::build_application()`, before `task_repository.schedule(&Task::UpdateYtdlp, ...)`, call `task_repository.list_non_completed()` and check whether any returned task's `task_type` equals `Task::UpdateYtdlp.task_type()`; only call `schedule` when none is found, and log at `info` level (with the existing task's id) when skipping. Verify by reading the diff against `design.md` - Decisions.
- [x] 1.2 Add a unit/integration test around `build_application()`'s seeding behavior (or the extracted guard logic, if pulled into a small testable function) covering: no existing `update_ytdlp` task → one is scheduled; an existing pending `update_ytdlp` task → no new one is scheduled; an existing running `update_ytdlp` task → no new one is scheduled. Verify with `cargo test <test name filter>`.
- [x] 1.3 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked` and confirm all pass.

## 2. Manual verification

- [x] 2.1 Run the daemon locally (`cargo build --release && ./target/release/yarrtube serve`) against a fresh sqlite file, confirm via logs or `sqlite3 <db> "select task_type, status from tasks"` that exactly one `update_ytdlp` row exists after startup.
- [x] 2.2 Stop and restart the daemon against the same database file, confirm via the same query that the `update_ytdlp` row count is still exactly one (i.e. the guard skipped re-seeding) and the log shows the skip message.
