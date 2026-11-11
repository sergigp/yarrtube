# Proposal

## Why

On the NAS (104 videos, 15 sources, a reconcile pass every 3 h) most info
lines only say that something started or ended, not what it did or how long
it took, and the minute-by-minute heartbeat carries no data. Richer end and
summary lines make the log usable for debugging after the fact.

## What Changes

Logging only: no task, event, retry or download behaviour changes. No
existing line is demoted to debug for now (the log is read with debug
filtered out); demoting the start/dispatch lines is deferred until the new
end lines have proven sufficient.

- **Tasks**: one info line when a task ends (`task finished`) with task id,
  type, the video / channel / playlist id from its payload, outcome (`done`,
  `unchanged`, `requeued` with next run and reason), run duration and time
  waited in the queue (claim time minus `run_at`). A failed attempt's
  existing warn (`task failed, retrying`) / error (`task failed permanently`)
  line is its end line and gains the same fields. `scheduled task`,
  `dispatching task` and the duplicate line stay at info.
- **Events**: one info line when an event ends (`event finished`) with id,
  type, payload, subscriber count, outcome and duration. Retry / dead-letter
  lines keep their level and gain the same fields. `published event` and
  `dispatching event` stay at info.
- **Reconcile pass** (channel and playlist): one summary line at the end
  (`reconciled channel` / `reconciled playlist`) with videos listed (absent
  when the listing failed), added, removed, syncs requested, orphan files
  deleted, listing duration and total duration. The start line and the
  per-video added / removed lines stay.
- **`sync_video`**: the handler returns a new task outcome, "nothing
  changed", which the executor handles like done (deletes the task) and
  reports as `outcome="unchanged"`, so no-op syncs are easy to filter out; no
  special-casing of task types by name. A sync that does something logs one
  info line (`synced video`) saying which: steps due (scheduled unless
  already queued), reset for redownload, metadata repaired, relocated.
- **Heartbeat**: every 5 minutes instead of every minute, carrying pending
  tasks per lane, running tasks, age of the oldest eligible pending task,
  pending events, dead-lettered tasks and events, and videos by download,
  summary and categorization status (pending, retrying, failed).
- **Categorization**: one info line per run (`categorized video`) with
  counts asked, yes, no, rejected; each `video categorized` line stays.
  `asked the model` gains `thinking` and `stopped_at_limit`. The
  summarizer's `model answered` gains `thinking` and `output_tokens` (it has
  `stopped_at_limit`).

### Lines replaced and where their information now lives

| Old line | Its information now on |
|---|---|
| `task done` (task_id) | `task finished` outcome=done (or unchanged) |
| `task re-queued` (task_id, run_at, reason) | `task finished` outcome=requeued, run_at, reason |
| `event done` (event_id) | `event finished` outcome=done |
| `yarrtube daemon is alive` (every minute) | `daemon heartbeat` every 5 minutes, with counts |

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `logging`: adds requirements for one end line per task and per event with
  outcome and timings, a reconcile pass summary, an `unchanged` outcome for
  no-op video syncs, and a per-video categorization summary.
- `daemon`: Heartbeat Logging now reports queue, dead-letter and video status
  counts on a 5-minute interval.

## Impact

- Code: `task_executor.rs`, `domain_events_consumer.rs`, `task_outcome.rs`,
  `task.rs`, `video_reconciler.rs`, `sync_video_task.rs`,
  `channel_video_reconciler.rs`, `playlist_video_reconciler.rs`,
  `internal_video_reconciler.rs`, `video_categorizer.rs`,
  `video_summarizer.rs`, `serve.rs`, a new heartbeat read service.
- Ports: `TaskRepository::list_dead_lettered`,
  `EventRepository::list_dead_lettered` and `VideoRepository::list` move from
  test-only inherent methods onto the ports (entity-returning reads, no
  count-style methods needed).
- Docs: `doc/AI.md` (what summaries and categorization log),
  `doc/INSTALLATION.md` (Logs: task/event end lines, reconcile summaries,
  heartbeat).
- No API, schema, dependency or configuration changes.
