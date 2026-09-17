## Context

See `proposal.md` - Why for the motivating bug. Relevant current-state facts:

- `serve.rs::build_application()` calls `task_repository.schedule(&Task::UpdateYtdlp, now + UPDATE_INTERVAL_SECONDS)` unconditionally on every startup (`serve.rs:290-295`), after `recover_stuck_tasks()` has already run.
- `SqliteTaskRepository::schedule` (`sqlite_task_repository.rs:166`) is a bare `INSERT`, with no uniqueness constraint or existence check on `task_type`.
- `UpdateYtdlpTask::handle` (`application/tasks/update_ytdlp_task.rs:51-53`) always reschedules itself an hour out, regardless of success/failure, and never dead-letters (`design.md` of `2026-09-14-resilient-ytdlp-lifecycle`) - so a chain seeded once persists and self-perpetuates for the lifetime of the database, independent of process restarts.
- `TaskRepository::list_non_completed()` already exists (`sqlite_task_repository.rs:19`, used today by `task_view_searcher.rs:48`) and returns every task in a `pending` or `running` state, across all task types, as `ScheduledTask` values that include `task_type: String`.
- This is a single-process daemon (`serve.rs::run`) against a single local SQLite file - no concurrent daemon instances race on `build_application()`.

## Goals / Non-Goals

**Goals:**
- Exactly one recurring `update_ytdlp` chain exists at a time, regardless of how many times the daemon restarts.
- No change to the recurring task's own scheduling cadence, retry/no-dead-letter semantics, or the immediate synchronous startup update.

**Non-Goals:**
- Cleaning up duplicate `update_ytdlp` rows already present in a deployed database from before this fix (see proposal.md - out of scope; handled manually).
- A general-purpose "singleton task type" mechanism in the task-scheduling capability. This stays local to `update_ytdlp`'s startup seeding, consistent with `reconcile_playlist`'s existing self-rescheduling pattern that this task already follows.
- Guarding against concurrent multi-process races, since only one daemon process runs against a given database at a time.

## Decisions

### Guard with `list_non_completed()`, filtered by task type, at the existing seed call site
In `build_application()`, before calling `task_repository.schedule(&Task::UpdateYtdlp, ...)`, call `task_repository.list_non_completed()` and check whether any returned task's `task_type` equals `Task::UpdateYtdlp.task_type()`. Schedule only if none is found; otherwise log at `info` level that an existing chain was found and skip.
- **Alternative considered**: add a dedicated repository method (e.g. `exists_non_completed(task_type: &str)`). Rejected - `list_non_completed()` already returns everything needed and is already used elsewhere for a similar read; adding a new repository method for one call site is unnecessary surface area.
- **Alternative considered**: enforce uniqueness at the schema level (e.g. a partial unique index on `task_type` for non-terminal rows). Rejected - would apply to every task type, but `download_video`, `sync_playlist`, etc. are intentionally allowed multiple concurrent pending rows; a blanket constraint would be wrong for those, and a per-type-conditional constraint is more machinery than this fix needs.
- **Alternative considered**: have the recurring task itself check before rescheduling (in `UpdateYtdlpTask::handle`). Rejected - the duplication is introduced only at startup seeding, not by the handler's self-reschedule (which always operates on a single task instance mid-poll); guarding there wouldn't address the actual source.

### Placement relative to `recover_stuck_tasks()`
The guard runs after `recover_stuck_tasks()` (already the case, since it's the same call site), so a task left `running` by an unclean shutdown is first recovered back to `pending` (or dead-lettered) before the non-completed check runs - it will already be visible to `list_non_completed()` as `pending`, correctly preventing a duplicate seed.

## Risks / Trade-offs

- **[Risk]** A database already carrying duplicate `update_ytdlp` chains (from restarts before this fix ships) keeps all of them running indefinitely - this fix only stops new ones from being added. → **Mitigation**: none built into this change (see proposal.md - out of scope); the operator clears the extra rows manually. Documented here so it isn't mistaken for an oversight.
- **[Risk]** If a future change makes `update_ytdlp`'s payload vary (it's currently always `{}`), the `task_type`-only check remains correct, since it only needs to know "is *a* recurring update already scheduled," not compare payloads. No action needed, noted for future readers.

## Migration Plan

- No data migration, no schema changes.
- Rollout is a normal build + redeploy; the next daemon start after deploying will find any already-seeded chain (if present) and skip re-seeding, or seed one as today if none exists.
- Rollback is a normal image/binary rollback; no forward-only state is introduced.
