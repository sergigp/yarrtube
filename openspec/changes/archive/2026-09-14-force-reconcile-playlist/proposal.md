## Why

Playlists only reconcile automatically once an hour. When a user adds a video
to a tracked YouTube playlist, or wants to confirm a filesystem heal happened,
they currently have no way to see it reflected sooner short of waiting out
the interval. They need a way to trigger a reconcile pass on demand from the
web app, alongside the existing delete action.

## What Changes

- Add an HTTP endpoint, `POST /api/playlists/{id}/reconcile`, that runs one
  reconcile pass for a playlist immediately and synchronously (YouTube
  membership diff for YouTube-linked playlists, filesystem healing for
  every playlist), via a new `VideoService::force_reconcile_playlist`
  method that shares its reconcile logic with the existing
  `reconcile_playlist` but does not touch the recurring reconcile schedule.
- Add a "Reconcile" action to the playlist actions menu in the web app,
  alongside the existing "Delete" action, calling the new endpoint.
- Because the on-demand path never schedules anything, triggering it
  repeatedly never queues duplicate future tasks: it always leaves whatever
  `ReconcilePlaylist` task is already pending (from creation or the last
  recurring pass) exactly as it was.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `playlist-reconciliation`: adds an on-demand trigger for a reconcile pass,
  exposed over HTTP, in addition to the existing creation-triggered and
  recurring-scheduled triggers.

## Impact

- **Backend**: `src/http/playlists/mod.rs` (new handler), `src/http/mod.rs`
  (new route), `src/domain/video/service.rs` (new
  `force_reconcile_playlist` method, sharing a private helper with the
  existing `reconcile_playlist`). No changes to `TaskRepository` or the
  background task/scheduling machinery.
- **Frontend**: `web/src/api.js` (new `reconcilePlaylist` call),
  `web/src/components/PlaylistActionsMenu.jsx` (new menu item).
- No database schema changes, no new domain events.
