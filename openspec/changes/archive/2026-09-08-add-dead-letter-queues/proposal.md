## Why

Today, both the `events` and `tasks` tables retry up to 5 times and then mark the row permanently `failed` — but that row is never deleted, never surfaced anywhere else, and nothing ever reads it again. The tables also never shrink: every row, whether it ends in `done` or `failed`, stays forever, so `events`/`tasks` are permanent status logs rather than queues. This was called out as an accepted known limitation when task scheduling was first introduced (`openspec/changes/archive/2026-09-08-add-task-scheduling/design.md`, "Known limitations": *"A permanently-dropped `sync_playlist` invocation ... silently ends that playlist's sync cycle until someone notices the log and intervenes"*). We want permanently-failed rows to land somewhere inspectable (a dead-letter table) instead of vanishing into a log line, and we want both tables to actually behave like queues: a row leaves the table once it reaches a terminal state, whether that's success or dead-letter.

## What Changes

- After the 5th failed attempt, an event or task is now moved to a new dead-letter table instead of being left in place with `status = 'failed'`: the terminal-failure logging that already happens SHALL continue unchanged, and in addition a row capturing the original id, type, payload, final error, attempt count, and timestamps SHALL be inserted into the dead-letter table.
- Two new tables are added, one per existing mechanism (mirroring the existing `events`/`tasks` split, kept separate for the same reasons that split exists — see `openspec/changes/archive/2026-09-08-add-task-scheduling/design.md` decision 1): `domain_events_dead_letter` and `tasks_dead_letter`.
- **BREAKING** (internal behavior, no external API today reads these tables): `events` and `tasks` become true queues — a row is deleted from its table as soon as it reaches a terminal state, whether that's successful completion (`mark_done`) or the 5th failure (moved to dead-letter). Rows no longer accumulate indefinitely with `status = 'done'` or `status = 'failed'`; a row's presence in `events`/`tasks` means it is still pending or retrying.
- Nothing changes about retry timing, the retry count threshold (still 5), subscriber fan-out for events, single-handler dispatch for tasks, or task crash recovery — this change only affects what happens once a row reaches a terminal state.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `domain-events`: "Retry Then Give Up" changes so the 5th failure moves the event to a new `domain_events_dead_letter` table instead of leaving it `status = 'failed'` in place; a new requirement establishes that the `events` table only ever holds pending/retrying rows (successful events are deleted, not just marked `done`).
- `task-scheduling`: "Retry With Fixed Delay Then Give Up" changes so the 5th failure moves the task to a new `tasks_dead_letter` table instead of leaving it `status = 'failed'` in place; a new requirement establishes that the `tasks` table only ever holds pending/retrying/running rows (successful tasks are deleted, not just marked `done`). "Crash Recovery" is unaffected in mechanism but now can itself trigger a dead-letter move if the recovered attempt was the task's 5th.

## Impact

- `src/infrastructure/repositories/sqlite_event_repository.rs`: `mark_failed_or_retry` and `mark_done` change their terminal-state behavior (dead-letter insert + delete, or delete, instead of an update); a new `domain_events_dead_letter` table and its own minimal repository/port for inserting into it.
- `src/infrastructure/repositories/sqlite_task_repository.rs`: same shape of change for `mark_failed_or_retry`, `mark_done`, and `recover_running` (which shares the failed-attempt path); a new `tasks_dead_letter` table and repository.
- `src/infrastructure/repositories/domain_events_consumer.rs` and `task_executor.rs`: no change expected to their control flow — they already just call `mark_done` / `mark_failed_or_retry` and trust the repository; this is mostly a repository-internal change, confirmed during design.
- `openspec/specs/domain-events/spec.md` and `openspec/specs/task-scheduling/spec.md`: requirements updated per the deltas above.
- No HTTP API, CLI surface, or external contract changes — everything here is internal persistence behavior.
