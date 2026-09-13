## Why

The SPA is functional but hard to use for day-to-day monitoring: the Tasks
tab shows a bare task type with no indication of which playlist or video it
concerns, the layout is capped narrow and buries the video player below a
flat video list, and there's no way to delete a playlist from the browser
even though the backend already supports it. This change addresses the
highest-value usability gaps surfaced during exploration.

## What Changes

- Reshape the app shell: a top tab bar (Playlists / Tasks), a header "Add
  Playlist" button that opens a modal dialog (built with a headless UI
  library — the app's first UI dependency), and a wider layout (no longer
  capped to 720px).
- Replace the inline create-playlist form with that modal dialog. Polling
  stays as the existing silent, fixed-interval `usePolling` per view — a
  header-level refresh control (spinner/countdown/manual button) was built
  and then dropped after review as unneeded complexity.
- Playlist list: show quality and source (kind) as chips.
- Wire up playlist deletion in the SPA against the existing
  `DELETE /api/playlists/{id}` endpoint: a "…" menu per list item and an
  action in playlist detail, both behind a confirmation dialog. Cascade
  deletion of the playlist's videos/files is a known backend gap, explicitly
  deferred to a separate change.
- Redesign playlist detail into a three-pane layout: video list sidebar
  (left), player (center, larger), video detail pane (right); collapses
  responsively on narrow viewports (sidebar becomes a drawer, detail becomes
  a bottom sheet/panel).
- Video detail: add a delete action and format all displayed timestamps as
  absolute values rounded to the minute.
- Playlist detail's video list: replace the plain status chip with an icon
  (none for `DOWNLOADED`, a download icon for `IN_PROGRESS`, a warning icon
  with a status-specific tooltip/label for `PENDING`/`ERRORED`/
  `ERRORED_RETRYING`) so a fully-downloaded video — the common case — reads
  as unremarkable at a glance.
- Tasks list: surface human-readable context (which playlist/video the task
  concerns), show status as a colored chip (running tasks stand out and sort
  to the top of the list) and retry count as a chip only when retries are
  greater than zero, and show the scheduled run time as a short relative
  string (e.g. "3s ago", "in 2m") instead of an absolute timestamp.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `task-listing`: the endpoint's response also includes the playlist name,
  and video title/id when applicable, resolved server-side at read time from
  the task's stored payload — not just the raw type/status/retries/run_at it
  returns today.

## Impact

- `web/src/**`: app shell, all components, `api.js`, `usePolling.js`, new
  dialog/menu components, CSS.
- `web/package.json`: new headless UI dependency (e.g. Radix primitives) for
  dialogs/menus.
- `src/http/tasks/dto.rs` and `src/http/tasks/mod.rs`: resolve playlist/video
  context into the response.
- `src/domain/task/*` and the playlist/video repositories: read-path lookups
  needed to resolve names from a task's payload.
- No change to `Video.updated_at` — confirmed during exploration that it
  reflects real lifecycle transitions (status changes, title refresh on
  sync, reconciliation resets), not just creation metadata, so it stays.
