## 1. Skill documentation

- [x] 1.1 Rephrase the `shared/` vs `repositories/` distinction in `SKILL.md`'s Infrastructure Layer section and file-structure template (`shared/` = infra usable across aggregates; `repositories/` = one aggregate's dedicated port implementation) and verify the wording is consistent with `design.md`'s Decisions.
- [x] 1.2 Codify the `<implementation>_<port>.rs` repository file-naming rule in `SKILL.md`'s File Structure and Naming section and verify it matches the renamed/moved files produced by section 2.

## 2. Repository file naming and placement

- [x] 2.1 Rename `infrastructure/repositories/video_file_repository.rs` to `filesystem_video_file_repository.rs` to match the `<implementation>_<port>` convention (implementation struct `FilesystemVideoFileRepository` unchanged), update `infrastructure/repositories/mod.rs` and all importers; verify `cargo build --release` succeeds.
- [x] 2.2 Split `infrastructure/repositories/sqlite_event_repository.rs` into an `EventPublisher` file and an `EventRepository` file (each with its Sqlite implementation and hand-written fake), and move both into a new `infrastructure/shared/domain_events/` module; update `infrastructure/shared/mod.rs` and `infrastructure/repositories/mod.rs`; verify `cargo build --release` succeeds. (Also dropped `EventRepository::insert_pending` and `SqliteEventRepository`'s `clock` field, both now dead: publishing goes exclusively through `EventPublisher`.)
- [x] 2.3 Move `infrastructure/repositories/system_clock.rs` to `infrastructure/shared/system_clock.rs`; update module declarations and every `use` path; verify `cargo build --release` succeeds.
- [x] 2.4 Update every importer of the moved/renamed items (`domain/playlist/service.rs`, `domain/video/service.rs`, `subscribers/*.rs`, `tasks/*.rs`, `serve.rs` composition root, and their test modules) to the new module paths; verify `cargo build --release` and `cargo test --locked` both succeed.

## 3. Simplify the eligible-task/event read

- [x] 3.1 Change `TaskRepository::list_eligible` to return `anyhow::Result<Vec<ScheduledTask>>` directly instead of `Vec<i64>`; update `SqliteTaskRepository`'s query and `FakeTaskRepository`; verify the existing `sqlite_task_repository.rs` tests still pass.
- [x] 3.2 Update `TaskExecutor::poll_once` to consume the entities directly, removing the per-id `find` call and the "disappeared before dispatch" branch; verify the existing `task_executor.rs` tests still pass. (Also removed `TaskRepository::find` from the trait — dead in production once the executor stopped calling it — keeping it as a test-only inherent method on `SqliteTaskRepository`.)
- [x] 3.3 Apply the same simplification to `EventRepository::list_eligible` and `DomainEventsConsumer`'s poll loop; verify the existing `domain_events_consumer.rs` and `sqlite_event_repository.rs`/relocated equivalent tests still pass. (Also removed `EventRepository::find` from the trait — same dead-in-production reasoning as `TaskRepository::find` — keeping it as a test-only inherent method on `SqliteEventRepository` and deleting `FakeEventRepository`'s copy outright since nothing called it any more.)
- [x] 3.4 Run `cargo clippy --all-targets --all-features --locked -- -D warnings` to confirm the removed branch leaves no dead-code or unused-import warnings.

## 4. Backfill task-level tests, then remove domain-level duplicates

- [x] 4.1 Add the missing scenarios to `tasks/download_video_task.rs`: errored-retrying with retries left, terminal errored on last attempt, no-op when the playlist no longer exists, no-op when the video no longer exists, and sanitized filename passed to the downloader; verify `cargo test download_video_task` passes.
- [x] 4.2 Add the missing scenarios to `tasks/delete_video_file_task.rs`: no-op when the playlist no longer exists, no-op when no matching file is found; verify `cargo test delete_video_file_task` passes.
- [x] 4.3 Delete the `#[cfg(test)] mod tests` block in `domain/video/service.rs` and verify `cargo test --locked` still passes with the same scenarios now exercised under `tasks/`.
- [x] 4.4 Delete the `#[cfg(test)] mod tests` block in `domain/playlist/service.rs` and verify `cargo test --locked` still passes (coverage already present in `http/playlists/mod.rs`).

## 5. Final verification

- [x] 5.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo build --release`, and `cargo test --locked` together and verify all four succeed with no regressions.
