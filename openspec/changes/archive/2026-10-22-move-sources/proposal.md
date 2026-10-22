# Proposal

## Why

A playlist's or channel's storage path is fixed at creation: the only way to
put its videos somewhere else is to delete and re-add it, which re-downloads
everything and loses watch state. Users reorganising their library (or
fixing a typo in a folder name) need to move a tracked source in place.

## What Changes

- New HTTP endpoints `POST /api/playlists/{id}/move` and
  `POST /api/channels/{handle}/move` taking a new storage `path`. They rename
  the source's show directory to the new path and record the new path,
  keeping every video record, episode number and watch state.
- A move is only accepted when the source is settled: every one of its videos
  is Downloaded, Errored or Excluded, and no download, thumbnail fetch, subtitle fetch
  or video file deletion for it is pending or running. Its pending recurring
  reconcile does not block the move.
- A move and a reconcile of the same source never overlap: a reconcile
  (recurring or on demand) waits for an in-flight move, and a move requested
  while a reconcile runs is rejected.
- The destination must be a valid storage path, neither inside nor containing
  any playlist's or channel's path (its own included), and must not exist on
  disk. Moves are same-filesystem renames only; a rename the
  filesystem cannot perform in place is rejected with nothing changed.
- When the Plex integration is enabled, a successful move asks Plex to scan
  the new show folder and the old one.
- Web UI: the edit playlist and edit channel dialogs show the source's
  current location and offer "Move…", which picks the new location with the
  same "Save to" + folder browser + folder name controls as the add dialogs.

## Capabilities

### New Capabilities
- `source-relocation`: moving a tracked playlist's or channel's show directory
  to a new storage path, its preconditions, its exclusion with reconciles,
  and the Plex scan it triggers.

### Modified Capabilities
- `web-ui`: the edit playlist and edit channel dialogs gain the current
  location and a "Move…" action.

## Impact

- Domain: a source mover service shared by playlists and channels; an
  in-memory per-source lock taken by the mover and by both reconcilers.
- Repositories: playlist and channel `update` used for the path; task
  repository queried for a source's pending/running per-video tasks; video
  file repository gains a directory rename.
- HTTP: two new routes in `src/application/http/playlists` and
  `src/application/http/channels`.
- Web: `EditPlaylistDialog`, `EditChannelDialog`, a new move dialog reusing
  `SaveToField`/`FolderBrowser`/`FolderNameField`, API client and types.
- No schema migration, no new task type, no new dependency.
