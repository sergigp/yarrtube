## Why

Every daemon start unconditionally seeds a new recurring `update_ytdlp` task chain (`serve.rs::build_application`), even when one is already pending or running from a previous start. Because the recurring task persists itself in SQLite and reschedules itself hourly forever by design, restarts never clean up an old chain — they only add another one alongside it. A NAS that restarts repeatedly (crash loops, redeploys, reboots) ends up with N independent hourly `yt-dlp` self-update loops running concurrently and growing without bound, which is exactly what's being observed in production.

## What Changes

- Before seeding the recurring `update_ytdlp` task at startup, check whether a non-terminal (pending or running) `update_ytdlp` task already exists via the existing `TaskRepository::list_non_completed()`.
- Skip scheduling and log at `info` level when one is already found; only seed when none exists.
- No change to the task's own self-rescheduling behavior, retry semantics, or the immediate synchronous startup update — only the initial seeding at daemon start becomes idempotent.
- Out of scope: no cleanup of duplicate chains already present in a deployed database from before this fix; those are cleared manually.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `ytdlp-self-update`: the "Automatic invocation at daemon startup" behavior for seeding the recurring update task becomes idempotent across restarts — it no longer creates a duplicate recurring chain when one is already scheduled.

## Impact

- `src/serve.rs` (`build_application`): add a non-terminal-task check before `task_repository.schedule(&Task::UpdateYtdlp, ...)`.
- No schema changes, no new repository methods (`list_non_completed` already exists and is used elsewhere).
- No change to `src/application/tasks/update_ytdlp_task.rs`'s self-rescheduling logic.
