## Why

`TaskRepository` and `EventRepository` currently encapsulate business behavior
(`mark_running`/`mark_done`/`mark_failed_or_retry`, retry-counting, dead-letter
thresholds) inside the repository implementations themselves, rather than in
the domain layer. This is a departure from this project's own convention
(`rust-architect` skill: repositories persist state, entities decide state
transitions) and it's about to be copied a third time by the upcoming
video-download work if not corrected first. Fixing it now, before adding a
`VideoRepository`, keeps the new code consistent with a single, deliberate
pattern instead of extending an inconsistency.

## What Changes

- Introduce a persisted `ScheduledTask` domain entity (id, task kind/payload,
  status, retries, `run_at`) with transition methods — `start()`, `complete()`,
  `fail(error, now)` — that themselves decide retry-vs-dead-letter based on the
  existing 5-attempt threshold and fixed retry delay. These thresholds move
  from `infrastructure/repositories/sqlite_task_repository.rs` constants into
  this domain logic.
- Reduce `TaskRepository` to CRUD: `schedule` (create), `find`, `list_eligible`
  (query), `update`, `delete`, plus a dead-letter insert — no more
  behavior-named methods. `TaskExecutor` changes from calling `mark_running`/
  `mark_done`/`mark_failed_or_retry` to `find` → call the entity's transition
  method → `update`/`delete`+dead-letter-insert.
- Mirror the same treatment for `EventRepository` with a persisted
  `ScheduledEvent` entity and `DomainEventsConsumer`.
- Recovery of tasks left `running` after a crash (`recover_running`) becomes a
  `list_running` query plus the same `fail(...)` transition applied per task,
  rather than a dedicated repository method.
- Rewrite the existing unit/behavior tests for both repositories and both
  executors against the new shape. Retry counts, backoff delay, and
  dead-letter thresholds are unchanged — this is a structural refactor with
  **no observable behavior change**.
- Update `.claude/skills/rust-architect/SKILL.md`'s CQS section to state the
  entity-mutation/CRUD-repository rule explicitly, so this pattern is the
  documented default going forward (captured as a task, applied during
  `opsx:apply`, not part of this proposal itself).

**BREAKING**: none externally — `TaskRepository`/`EventRepository` are
internal ports with a single production implementation each; no HTTP/CLI
surface changes.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
(none — retry/backoff/dead-letter/crash-recovery behavior for both tasks and
events is unchanged; only the internal implementation shape changes. This
change sets `skip_specs: true`.)

## Impact

- `src/domain/task/`: new `ScheduledTask` entity (or similar file), transition
  methods, retry/backoff constants move here.
- `src/domain/event.rs` (or a new `src/domain/event/` folder): new
  `ScheduledEvent` entity, mirrored transition methods.
- `src/infrastructure/repositories/sqlite_task_repository.rs`: trait and
  implementation reduced to CRUD; schema unchanged.
- `src/infrastructure/repositories/sqlite_event_repository.rs`: same.
- `src/infrastructure/repositories/task_executor.rs`: dispatch loop rewritten
  around find/transition/update instead of mark_* calls.
- `src/infrastructure/repositories/domain_events_consumer.rs`: same.
- `src/subscribers/sync_playlist_on_playlist_created.rs`,
  `src/tasks/sync_playlist_task.rs`: no behavior change, but their tests use
  `FakeTaskRepository`/fakes that need updating to the new trait shape.
- `.claude/skills/rust-architect/SKILL.md`: CQS section updated.
- No database schema changes (same `tasks`, `tasks_dead_letter`, `events`,
  `events_dead_letter` tables).
