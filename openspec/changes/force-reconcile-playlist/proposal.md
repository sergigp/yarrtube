## Why

Playlists only reconcile automatically once an hour. When a user adds a video
to a tracked YouTube playlist, or wants to confirm a filesystem heal happened,
they currently have no way to see it reflected sooner short of waiting out
the interval. They need a way to trigger a reconcile pass on demand from the
web app, alongside the existing delete action.

## What Changes

- Add an HTTP endpoint, `POST /api/playlists/{id}/reconcile`, that runs one
  reconcile pass for a playlist immediately and synchronously, reusing the
  existing `VideoService::reconcile_playlist` logic (YouTube membership diff
  for YouTube-linked playlists, filesystem healing for every playlist, and
  scheduling the next recurring reconcile) unchanged.
- Add a "Reconcile" action to the playlist actions menu in the web app,
  alongside the existing "Delete" action, calling the new endpoint.
- The pre-existing recurring reconcile task for that playlist (scheduled by
  the prior pass) is left as-is; it still fires later. No task
  cancellation/dedup is introduced — reconcile is idempotent, so a redundant
  future pass is harmless.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `playlist-reconciliation`: adds an on-demand trigger for a reconcile pass,
  exposed over HTTP, in addition to the existing creation-triggered and
  recurring-scheduled triggers.

## Impact

- **Backend**: `src/http/playlists/mod.rs` (new handler), `src/http/mod.rs`
  (new route). No changes to `VideoService`, `TaskRepository`, or the
  background task/scheduling machinery.
- **Frontend**: `web/src/api.js` (new `reconcilePlaylist` call),
  `web/src/components/PlaylistActionsMenu.jsx` (new menu item).
- No database schema changes, no new domain events.
