## Context

See `proposal.md` - Why. Today `SqliteTaskRepository` and `SqliteEventRepository`
each own their retry/backoff/dead-letter *decisions* (`MAX_ATTEMPTS`,
`RETRY_DELAY_SECONDS`, the running/pending/dead-letter transitions) inside
`mark_running`/`mark_done`/`mark_failed_or_retry`(/`recover_running` for
tasks). `TaskExecutor` and `DomainEventsConsumer` just call these behavior-
named methods and branch on the `anyhow::Result` they get back. Both
`Task` (`src/domain/task/task.rs`) and `DomainEvent` (`src/domain/event.rs`)
are pure "kind + payload" value types today — the retry/status/`run_at`
lifecycle state exists only in the infra DTOs `PersistedTask`/`PersistedEvent`.

## Goals / Non-Goals

**Goals:**
- Move every retry/backoff/dead-letter *decision* into domain entities
  (`ScheduledTask`, `ScheduledEvent`), leaving `TaskRepository`/
  `EventRepository` as CRUD ports: create, `find`, list queries, `update`,
  `delete`, plus one atomic "move to dead letter" write.
- Preserve retry count (5), fixed retry delay, and dead-letter behavior
  exactly — this is a structural refactor, not a behavior change.
- Update `rust-architect`'s CQS section so this is the documented default,
  not a one-off.

**Non-Goals:**
- No database schema changes (`tasks`, `tasks_dead_letter`, `events`,
  `events_dead_letter` keep their current columns).
- No change to `PlaylistRepository` (already CRUD-shaped:
  `find`/`insert_with_event`/`delete_with_event`/`list`).
- No new crash-recovery behavior for events (there isn't any today — only
  tasks recover a stuck `running` row on startup — and this change doesn't
  add it).
- No shared abstraction/trait between `ScheduledTask` and `ScheduledEvent`
  (see Decisions).
- Does not touch the video-download feature; that is a separate, later
  change built on top of this one.

## Decisions

**One persisted entity per aggregate, not a shared "Retryable" abstraction.**
`ScheduledTask` and `ScheduledEvent` end up structurally identical (id, kind,
payload, retries, timestamps, a `fail(error, now)` transition that decides
retry-vs-dead-letter). They are still two separate types with their own
`MAX_ATTEMPTS`/retry-delay constants, not a shared generic/trait. Unifying
them would couple two independent aggregates' retry policy together for a
resemblance that's coincidental today; if it diverges later (e.g. events gain
crash recovery, or a different backoff curve), a shared abstraction would
have to be undone. Duplication here is cheap and each copy is ~20 lines.

**Entities carry a raw `i64` id, not a value object.** Unlike `PlaylistId`/
`VideoId`, this id is an autoincrement persistence handle with no business
meaning outside "which row is this" — consistent with how `PersistedTask`/
`PersistedEvent` already expose it. No `TaskId`/`EventId` newtype introduced.

**Transition methods consume and return `Self` (or an outcome enum), they
don't mutate in place.** Matches the existing functional style
(`Video::create` builds a new value; nothing in this codebase does `&mut
self`). `fail()` specifically returns an outcome enum rather than `Self`,
because failing can produce two different *kinds* of next state:

```rust
pub enum TaskFailureOutcome {
    Retry(ScheduledTask),        // still has attempts left
    DeadLetter(DeadLetteredTask), // exhausted retries
}

impl ScheduledTask {
    pub fn start(self, now: DateTime<Utc>) -> Self { /* -> running */ }
    pub fn fail(self, error: impl Into<String>, now: DateTime<Utc>) -> TaskFailureOutcome { /* ... */ }
}
```

`TaskExecutor` (and `DomainEventsConsumer`) pattern-match the outcome and
call `repository.update(...)` or `repository.dead_letter(...)` accordingly —
the repository never decides which one to call, it's just told.

**Success is a `delete`, not a status.** There's no persisted "done" state
today (`mark_done` deletes the row) and nothing changes that — success just
calls `repository.delete(id)`, no entity transition needed for it.

**`ScheduledEvent` has no `start()`/running state.** Tasks mark `running`
before dispatch (so a crash mid-handler can be recovered on restart);
`DomainEventsConsumer` never did this for events, and there is no event
crash-recovery requirement today. This asymmetry is preserved as-is —
introducing running-state tracking for events would be a behavior addition,
out of scope here.

**Crash recovery becomes a query, not a repository verb.** `recover_running()`
is replaced by `TaskRepository::list_running()` (a plain read) plus applying
`.fail(...)` to each result and writing it back — the "treat a stuck running
task as a failed attempt" *decision* moves to whatever calls this at startup
(the same place that already runs a heartbeat/executor).

**`TaskExecutor`/`DomainEventsConsumer` gain a `Clock` dependency.** Because
timestamping (`updated_at`, `run_at` on retry, `failed_at` on dead-letter) now
happens inside the entity's transition methods rather than inside the SQL
layer, whatever calls those methods needs to supply `now`. Both already have
a natural place to take `Arc<dyn Clock>` (the same port `VideoService` and
`SqliteTaskRepository` already use) — wired up in
`serve.rs::build_application`.

**Dead-lettering stays one atomic repository method.** `dead_letter(&self,
task: &DeadLetteredTask)` inserts into the dead-letter table and deletes the
original row in one transaction — mirroring
`PlaylistRepository::insert_with_event`/`delete_with_event`, which already
establishes the precedent of one repository method for an atomic
multi-statement write, rather than forcing that into two CQS calls and
losing atomicity.

## Risks / Trade-offs

- [Risk] A `find` → mutate → `update` sequence opens a race window between
  the read and the write that the old single-statement `UPDATE ... SET
  retries = retries + 1` didn't have. → Mitigation: both `TaskExecutor` and
  `DomainEventsConsumer` are single sequential polling loops today (no
  concurrent pollers), and `Sqlite{Task,Event}Repository` serialize all
  access behind one `Mutex<Connection>` already — no new concurrent access
  pattern is introduced. Flagging this per the CQS-split cost called out in
  `rust-architect`, not silently absorbing it.
- [Risk] This touches already-shipped, tested infrastructure with no
  behavior change to show for it if something regresses. → Mitigation: port
  every existing test scenario (retry count, backoff timing, dead-letter
  threshold, crash recovery) to the new shape rather than dropping any;
  `cargo test --locked` must pass unchanged in outcome before this is done.
- [Risk] Duplicating the retry/dead-letter shape across `ScheduledTask` and
  `ScheduledEvent` means a future policy change (e.g. exponential backoff)
  has to be made twice. → Mitigation: accepted (see Decisions) — premature
  unification is the worse failure mode here.

## Migration Plan

No data migration: table schemas are unchanged, so this ships as a normal
code release. Rollback is a plain revert — there's no straddling state
between old and new repository shapes to reconcile.

## Open Questions

- Exact file layout for the new entities (e.g. whether `DeadLetteredTask`
  gets its own file or lives alongside `ScheduledTask` in
  `domain/task/scheduled_task.rs`, and whether `src/domain/event.rs` becomes
  a `domain/event/` folder to hold both `DomainEvent` and `ScheduledEvent`)
  is left to implementation, following this project's file-per-concept
  convention — it doesn't change the approach or the task breakdown.
