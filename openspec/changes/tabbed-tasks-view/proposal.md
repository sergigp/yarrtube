## Why

The tasks view is a single flat list of every pending and running task. On a
busy instance the sync (reconcile) tasks drown out the things worth watching —
active downloads and failing tasks — and there is no way to narrow the list or
to see *why* a retrying task keeps failing.

## What Changes

- Group the tasks view into tabs by task family: **Active** (anything running),
  **Downloads**, **Syncs**, **Cleanup**, and **All**. The view opens on
  **Active** so what's happening now is the default, not the sync noise.
- Each tab shows a live count of its tasks.
- Add a search field (mirroring the sidebar's) that filters the current tab by
  the task's human description, shown only once a tab's list is long enough to
  need it.
- Improve each task row: give **every** task type a plain-language description
  (including `reconcile_plex_collections`, which currently renders its raw
  type string) and never fall back to a raw snake_case type; surface a
  retrying task's last error, which today is returned by the API but never
  shown.
- Give each task family a small icon so rows are scannable.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `web-ui`: the tasks view gains tab grouping by task family, per-tab counts,
  task search, full human-language task descriptions, and display of a task's
  last error.

## Impact

- Frontend only. No backend, API, or DTO changes — all required data
  (`task_type`, `status`, `last_error`, resolved payload names) is already in
  the `GET /api/tasks` response.
- Code: `web/src/components/TasksView.tsx`, `web/src/lib/tasks.ts`, and their
  colocated tests. Uses existing UI primitives (`tabs.tsx`, `tooltip.tsx`,
  `input.tsx`); no new dependencies.
