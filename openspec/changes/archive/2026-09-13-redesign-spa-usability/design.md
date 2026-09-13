## Context

See proposal.md for motivation. Current state relevant to the approach:

- `web/src/App.jsx` renders two tabs by swapping components; there is no
  header action area and no modal/dialog anywhere in the app.
- Each view (`PlaylistList`, `TasksView`, `PlaylistDetail`) calls the shared
  `usePolling` hook independently, each running its own `setInterval(3000)`
  with no visible feedback and no manual trigger.
- `PlaylistDetail` renders the video list and the selected video's detail
  stacked vertically, not side-by-side.
- `GET /api/tasks` returns `TaskResponse` (`src/http/tasks/dto.rs`) built
  directly from `ScheduledTask`, dropping the JSON `payload` column entirely
  — the playlist/video a task concerns is not exposed today.
- `DELETE /api/playlists/{id}` already exists and works end-to-end
  server-side; nothing in the SPA calls it.
- The app has zero UI/component dependencies today — plain JSX + hand-written
  CSS in `App.css`.

## Goals / Non-Goals

**Goals:**
- Give the Tasks tab enough context (which playlist/video, status, retries as
  chips) to be useful for monitoring without cross-referencing IDs by hand,
  and make the currently-running task easy to spot at a glance.
- Make the playlist detail view video-first: bigger player, video list and
  detail as side panels, usable down to phone width via a responsive
  collapse.
- Let a playlist be deleted from the browser using the endpoint that already
  exists.
- Introduce one shared dialog/menu primitive (Radix) rather than one-off
  modal implementations per feature.

**Non-Goals:**
- Cascading playlist deletion to its videos/files — tracked as a separate,
  explicitly deferred change.
- Any change to `Video.updated_at` or video lifecycle semantics.
- Any change to task scheduling behavior (retry count, backoff, statuses).
- Multi-user auth/permissions, or supporting more than one connected client
  meaningfully differently than today.

## Decisions

### Headless UI library: Radix primitives, not native `<dialog>`
Use `@radix-ui/react-dialog` (Add Playlist form, delete confirmations) and
`@radix-ui/react-dropdown-menu` (the playlist "…" action menu), added as the
app's first UI dependencies. This change needs three distinct dialog-shaped
interactions (create-playlist form, playlist delete confirmation, video
delete confirmation) plus one menu, so paying for accessible focus-trapping
and consistent animation once, via unstyled primitives we skin with the
app's existing plain CSS, is worth the new dependency. A native `<dialog>`
was considered and rejected — acceptable for a single one-off modal, but
would mean re-solving focus management and outside-click handling by hand
for each of the three+ dialogs this change introduces.

### Refresh cadence: silent per-view polling, no header control
Each view keeps its own `usePolling(fetcher, deps)` call, each running its
own plain `setInterval(3000)` — the original behavior. A header-level
refresh control (a shared clock exposing seconds-remaining/spinner state via
context, driving `usePolling` and a manual "tick now" button) was built and
manually verified, then removed after review: it added a context provider,
a new component, and cross-view state purely to expose a cadence that's
fixed and short enough not to need surfacing. Nothing currently depends on
knowing when the next poll fires.

### Task context resolution: read-time lookups, batched
`list_tasks`'s handler decodes each task's stored `payload` (the existing
per-variant `decode_*` helpers on `Task`) to recover `playlist_id` and,
where applicable, `video_id`. It then resolves names by looking up the
distinct set of referenced playlists (and videos) once per request — not
once per task — to avoid N+1 queries. A playlist or video that can no longer
be found (e.g. deleted since the task was scheduled) resolves to an absent
name rather than failing the request, per the task-listing spec delta.

### Playlist deletion UI: "…" menu + confirmation, no cascade handling
A Radix dropdown menu on each playlist list item exposes "Delete"; the
playlist detail view exposes the same action (e.g. in its header). Both
route through the same confirmation dialog before calling
`DELETE /api/playlists/{id}`. The dialog's copy will not claim files are
cleaned up, since the backend does not do that today — see Non-Goals.

### Timestamp formatting: absolute for video detail, relative for tasks
A shared `formatDateTime` helper (absolute date + time, seconds dropped, no
relative/humanized phrasing) replaces the raw ISO timestamps in
`VideoDetail`. The tasks list uses a separate `formatRelativeTime` helper
instead (`"3s ago"`, `"in 2m"`): a task's `run_at` is only useful relative to
now (is it about to run, overdue, already retried a while ago), so an
absolute clock time forces the reader to do the subtraction themselves.

### Task list status chips: colored, retries hidden at zero, running first
The status chip is colored (accent-tinted for `running`, muted for
`pending`) so an in-progress task is visible without reading the text, and
the list sorts `running` tasks before `pending` ones so they surface at the
top rather than wherever they happen to fall in fetch order. The retries
chip is omitted entirely when `retries === 0`, since a task on its first
attempt has nothing noteworthy to report.

### Video list status: icon instead of a text chip
`PlaylistDetail`'s video list shows nothing for `DOWNLOADED` (the expected,
common case), a download icon for `IN_PROGRESS`, and a warning icon for
`PENDING`/`ERRORED`/`ERRORED_RETRYING` — each icon carries a status-specific
`title`/`aria-label` (e.g. "This video failed to download and will be
retried.") rather than a generic label, so hovering or a screen reader gets
an accurate description instead of a reused "pending" string.

## Risks / Trade-offs

- [Risk] New UI dependency increases bundle size and introduces a pattern
  not used elsewhere in the app → Mitigation: pull in only the specific
  Radix primitive packages needed (dialog, dropdown-menu), not a full
  component kit, and style them with the existing plain CSS conventions.
- [Risk] Resolving playlist/video names on every poll of `GET /api/tasks`
  adds repository calls to a frequently-hit endpoint → Mitigation: batch
  lookups per request; the data set (tracked playlists/videos for a single
  daemon instance) is small enough that this is not expected to matter in
  practice.
- [Risk] Responsive collapse of the three-pane playlist view adds real
  interaction complexity (drawer + bottom panel state) for a small tool →
  accepted, since the user explicitly wants it usable on narrow viewports.
- [Risk] Shipping playlist delete without cascade cleanup means deleting a
  playlist from the browser silently orphans its video rows and downloaded
  files on disk → Mitigation: called out explicitly in the proposal and in
  tasks.md as deferred, not hidden from the user experience design; consider
  surfacing a note in the confirmation dialog copy during implementation.

## Migration Plan

Purely additive: new UI, new response fields, no schema or data migration.
Ships as a normal release. Rollback is a plain revert — no persisted state
depends on the new behavior.
