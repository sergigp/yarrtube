## 1. Task domain entity

- [ ] 1.1 Add a `ScheduledTask` entity (id, task_type, payload, status
      [Pending|Running], retries, run_at, created_at, updated_at) under
      `src/domain/task/`, plus a `DeadLetteredTask` type for the dead-letter
      shape, and verify with unit tests covering `start(now)` transitioning
      to Running.
- [ ] 1.2 Add `ScheduledTask::fail(error, now) -> TaskFailureOutcome`
      (`Retry(ScheduledTask)` | `DeadLetter(DeadLetteredTask)`), with the
      5-attempt threshold and fixed retry delay moved here as domain
      constants, and verify with unit tests: below-threshold failure returns
      `Retry` with incremented retries and `run_at` pushed out by the fixed
      delay; the 5th failure returns `DeadLetter`.

## 2. Event domain entity

- [ ] 2.1 Add a `ScheduledEvent` entity (id, event_type, payload, retries,
      created_at, updated_at) and a `DeadLetteredEvent` type, mirroring
      `ScheduledTask` but with no `start()`/running status (events have no
      crash-recovery today), and verify with unit tests for
      `fail(error, now)` returning `Retry`/`DeadLetter` at the same 5-attempt
      threshold.

## 3. TaskRepository as CRUD

- [ ] 3.1 Change `TaskRepository` trait to `schedule` (create), `find(id)`,
      `list_eligible()`, `list_running()`, `update(&ScheduledTask)`,
      `delete(id)`, `dead_letter(&DeadLetteredTask)` (atomic insert +
      delete), removing `mark_running`/`mark_done`/`mark_failed_or_retry`/
      `recover_running`.
- [ ] 3.2 Reimplement `SqliteTaskRepository` against the new trait, dropping
      the now-unused `MAX_ATTEMPTS`/`RETRY_DELAY_SECONDS` constants and
      `apply_failed_attempt`, and verify by porting every existing
      `sqlite_task_repository` test (schedule-then-list-eligible, retry
      reschedule, 5th-failure dead-letter, crash-recovery-on-restart) to the
      new shape with `cargo test sqlite_task_repository`.

## 4. EventRepository as CRUD

- [ ] 4.1 Change `EventRepository` trait to `insert_pending` (create),
      `find(id)`, `list_eligible()`, `update(&ScheduledEvent)`, `delete(id)`,
      `dead_letter(&DeadLetteredEvent)`, removing `mark_done`/
      `mark_failed_or_retry`.
- [ ] 4.2 Reimplement `SqliteEventRepository` against the new trait, and
      verify by porting every existing `sqlite_event_repository` test
      (list-eligible, retry-below-threshold, 5th-failure dead-letter) to the
      new shape with `cargo test sqlite_event_repository`.

## 5. Executor and consumer rewrite

- [ ] 5.1 Add a `Clock` dependency to `TaskExecutor`, rewrite `poll_once` to
      `find` → `start(now)` → `update` before dispatch, then on the
      handler's result either `delete` (success) or match `fail(error, now)`
      into `update`/`dead_letter` (failure), and verify with
      `cargo test task_executor` (dispatch-and-succeed,
      dispatch-and-retry, dispatch-and-dead-letter after 5 attempts).
- [ ] 5.2 Replace `recover_running()` with a startup routine (in
      `TaskExecutor` or `serve.rs`) that calls `list_running()` and applies
      `fail(...)` to each, and verify with a test that a task left `running`
      from a previous process is retried or dead-lettered exactly as
      `mark_failed_or_retry` did before.
- [ ] 5.3 Add a `Clock` dependency to `DomainEventsConsumer`, rewrite
      `poll_once` to `find`/dispatch → `delete` (success) or `fail(error,
      now)` → `update`/`dead_letter` (failure), and verify with
      `cargo test domain_events_consumer`.

## 6. Composition root

- [ ] 6.1 Update `serve.rs::build_application` to construct
      `TaskExecutor`/`DomainEventsConsumer` with the shared `Clock`, wire up
      the new startup recovery call, and verify `cargo build --release`
      succeeds and `serve` starts cleanly against a fresh SQLite file.

## 7. Documentation

- [ ] 7.1 Update `.claude/skills/rust-architect/SKILL.md`'s CQS section to
      state explicitly that repositories are CRUD (create/find/list/update/
      delete) and business-meaningful state transitions belong on the
      entity, with `mark_running`/`mark_done`-style methods called out as
      the anti-pattern this project moved away from, and verify by reading
      the updated section back for consistency with this change's actual
      shape.

## 8. Full verification

- [ ] 8.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets
      --all-features --locked -- -D warnings`, and `cargo test --locked`,
      and confirm all pass with no behavior change to existing scenarios
      (same retry counts, backoff delay, dead-letter thresholds for both
      tasks and events).
