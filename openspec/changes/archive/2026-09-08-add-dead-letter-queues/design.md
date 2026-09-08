## Context

See `proposal.md` - Why for motivation. Relevant current state this design builds on:

- `events` and `tasks` are each a single SQLite table (`CREATE TABLE IF NOT EXISTS`, no migrations framework) with `status`, `retries`, and `last_error` columns. Rows are never deleted today; `status` transitions `pending → done` or `pending → failed` and stays there forever.
- `MAX_ATTEMPTS = 5` is a private `const` in each repository file (`sqlite_event_repository.rs`, `sqlite_task_repository.rs`) — same value, defined twice, no shared constant.
- The give-up decision (increment retries, compare to `MAX_ATTEMPTS`, choose the terminal state) already lives entirely inside the SQLite repositories' `mark_failed_or_retry`/`apply_failed_attempt`, not in the domain layer or in `DomainEventsConsumer`/`TaskExecutor` — both of those just call `mark_done` or `mark_failed_or_retry` and trust the repository. This is a pre-existing, accepted deviation from the project's usual "business decisions belong in the domain service" convention (see `rust-architect` skill), not something this change introduces.
- Every write happens through one `Arc<Mutex<Connection>>`, one statement per call, each wrapped in `anyhow::Context`. `SqlitePlaylistRepository` also writes directly into the `events` table (`insert_pending_row`) to keep a playlist write and its event insert in one transaction.
- `PersistedEvent`/`PersistedTask` (the DTOs handed to the consumer/executor) intentionally expose only `id`, `event_type`/`task_type`, and `payload` — no `retries` or `status`. This change doesn't need to change that: the terminal-state decision stays inside the repository.

## Goals / Non-Goals

**Goals:**
- Give both mechanisms a real dead-letter table so a permanently-failed row is inspectable (by direct SQLite query, e.g. via `sqlite3` CLI against the mounted volume) instead of only a log line.
- Make `events`/`tasks` real queues: a row's existence in the table means "still pending or retrying"; nothing lingers after a terminal state.
- Keep the change repository-internal: no change to `EventRepository`/`TaskRepository`'s public method signatures, `DomainEventsConsumer`/`TaskExecutor` control flow, or any HTTP/CLI surface.

**Non-Goals:**
- No read API (HTTP endpoint, CLI command, or new port) for inspecting dead-lettered rows in this change — direct SQL access is the interim story, matching how `status = 'failed'` rows are inspected today.
- No retention/cleanup policy for the dead-letter tables themselves — they can grow unboundedly, same as `events`/`tasks` could before this change. Accepted as out of scope; dead-letter rows are expected to be rare (only permanent failures) so this is a much slower-growing concern than the problem this change fixes.
- No change to `MAX_ATTEMPTS` (still 5), the fixed 30s task retry delay, event fan-out semantics, or crash recovery's mechanism.
- Not unifying the `events`/`tasks` split into one generic table — out of scope here, and already a deliberate decision from the original design (kept for the same extractability reason).

## Decisions

### 1. Dead-letter insert + source-row delete stay inside the existing repository methods, as one SQLite transaction
`SqliteEventRepository::mark_failed_or_retry` and `SqliteTaskRepository::apply_failed_attempt` (used by both `mark_failed_or_retry` and `recover_running`) keep their current signatures. When the incremented retry count reaches `MAX_ATTEMPTS`, instead of one `UPDATE ... SET status = 'failed'`, the method now runs two statements — `INSERT INTO <table>_dead_letter (...)` then `DELETE FROM <table> WHERE id = ?` — wrapped in an explicit `rusqlite` transaction (`conn.transaction()`) so the two writes commit or fail together.
**Why:** this is the smallest change that satisfies the new requirement while preserving the existing architecture: no new port is injected into `DomainEventsConsumer`/`TaskExecutor`, and the give-up decision stays exactly where it already lives (see Context). The transaction avoids the split-read-then-write race the `rust-architect` skill warns about — both writes happen under the single `Mutex<Connection>` guard already held for the call, inside one `rusqlite` transaction, so a mid-write crash can't leave a row duplicated in both tables or dropped from both.
**Alternative considered:** introduce a separate `DeadLetterRepository` port injected into each repository or into the consumer/executor. Rejected — nothing else needs to write dead-letter rows, there's no read path in this change to justify a port abstraction, and it would require passing a second repository into `SqliteEventRepository`/`SqliteTaskRepository`'s constructors for no behavioral benefit.

### 2. `mark_done` becomes a `DELETE`, not an `UPDATE ... SET status = 'done'`
Both `mark_done` implementations change from updating `status` to `DELETE FROM events/tasks WHERE id = ?1`. `list_eligible`'s query (`WHERE status = 'pending'` / `WHERE status = 'pending' AND run_at <= ?1`) is unchanged — a deleted row simply can't match it, so no query changes are needed there.
**Why:** this is the direct implementation of "the table behaves like a queue" — the simplest way to guarantee a successfully-processed row doesn't linger is to remove it, rather than adding a new status value and filtering it out everywhere `events`/`tasks` are queried.

### 3. Dead-letter table schema mirrors the source row plus a `failed_at` timestamp; no foreign key back to the source table
```
domain_events_dead_letter: id, original_event_id, event_type, payload, retries, last_error, created_at, failed_at
tasks_dead_letter:         id, original_task_id,  task_type,  payload, retries, last_error, created_at, failed_at
```
- `id`: new `INTEGER PRIMARY KEY AUTOINCREMENT` for the dead-letter table itself.
- `original_event_id`/`original_task_id`: the id the row had in `events`/`tasks` — kept for traceability, but **not** a foreign key, since the source row is deleted as part of the same transaction that creates this one (nothing to reference).
- `retries`: the final attempt count (always `MAX_ATTEMPTS` under current logic, but stored rather than hardcoded so this stays correct if `MAX_ATTEMPTS` ever changes).
- `created_at`: carried over from the source row (when it was originally published/scheduled), so age-since-creation is visible without cross-referencing anything.
- `failed_at`: `self.clock.now()` at the moment of the transition — when it entered the dead letter, distinct from `created_at`.
**Why:** matches the existing table style (flat columns, `TEXT` timestamps via `to_rfc3339()`, no foreign keys elsewhere in the schema either) and carries enough information to diagnose a permanent failure without needing the now-deleted source row.

### 4. Table creation follows the existing `CREATE TABLE IF NOT EXISTS` pattern, added to each repository's constructor
`SqliteEventRepository::new` and `SqliteTaskRepository::new` each gain one more `CREATE TABLE IF NOT EXISTS <name>_dead_letter (...)` call alongside their existing table creation, guarded by the same connection lock.
**Why:** consistent with how `events`/`tasks` (and `videos`, `playlists`) are already created — no migrations framework exists in this project, and introducing one is out of scope for this change.

### 5. `MAX_ATTEMPTS` stays a private per-file constant, duplicated as it is today
Not consolidating the two `MAX_ATTEMPTS = 5` constants into a shared one.
**Why:** out of scope — this change doesn't touch retry-count logic, only what happens once the existing threshold is reached. Consolidating is a separate, unrelated cleanup.

## Risks / Trade-offs

- **Two-statement write inside the transition path** (previously one `UPDATE`) → mitigated by wrapping both statements in one `rusqlite` transaction, per Decision 1; if the transaction fails, `mark_failed_or_retry`/`apply_failed_attempt` returns an error and the row is left exactly as it was (still in the source table, un-transitioned) — same failure-atomicity guarantee as today's single-statement version.
- **Existing unit tests assert `status = 'failed'` still readable by querying the source table after the 5th failure** (e.g. `it_should_drop_a_recovered_running_task_once_the_attempt_limit_is_exceeded` in `sqlite_task_repository.rs`) — these will fail to compile/pass once the row is deleted instead of updated. They need rewriting to instead assert the row is gone from the source table and present in the dead-letter table. Tracked in `tasks.md`.
- **Dead-letter tables have no retention policy** and will grow forever, same as `events`/`tasks` did before this change → accepted as a Non-Goal; permanent failures are expected to be rare relative to normal throughput, so this is a much smaller-scale version of the problem this change fixes, not a new one.
- **No read path for dead-lettered rows beyond direct SQL** → accepted Non-Goal; if this becomes a recurring operational need, a follow-up change can add an HTTP/CLI surface without touching the schema decided here.

## Migration Plan

- Both new tables are created with `CREATE TABLE IF NOT EXISTS` at repository construction time, same as every other table in this project — no manual migration step.
- No existing endpoint, CLI command, or external contract changes shape. The only externally-observable difference is that `events`/`tasks` rows that used to stay `status = 'done'`/`'failed'` forever now disappear once terminal (queried today only via ad hoc SQL against the mounted SQLite file, not through any API) — and permanently-failed rows are now findable in the new dead-letter tables instead.
- Rollback is a plain revert: the new tables sit unused in the SQLite file (itself not persisted across container recreation per the `daemon` spec) — nothing to clean up. Any dead-letter rows written before a rollback are simply orphaned data, same as any other unused table would be.
