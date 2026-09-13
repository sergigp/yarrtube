## Why

Deleting a playlist today only removes its row from the `playlists` table. Every
video record for it stays in SQLite forever, every downloaded file stays on
disk, and the playlist's folder is never removed. The SPA already has a
working delete action (confirm dialog, API call) whose own copy admits the
gap: *"Its downloaded video files and records are not cleaned up
automatically."* This change closes that gap end-to-end.

## What Changes

- Add a uniqueness rule: creating a playlist (YouTube-linked or custom) is
  rejected with 400 if its `path` is already used by a different existing
  playlist. This is a real, independently useful fix — two playlists
  currently sharing a `path` would already cause `reconcile_filesystem` to
  treat each other's downloaded files as orphans and thrash them. It's also
  the precondition that makes the next point safe.
- `delete_playlist` synchronously deletes every video row for that playlist
  (new bulk-delete repository method), in the same use case as deleting the
  playlist row, so a video record can never outlive its playlist — not even
  for the few seconds it'd take an event/task poll cycle to catch up.
- `PlaylistDeleted` gains a `path` field, since by the time any subscriber
  reacts to it, the playlist row (and thus its path) is already gone from
  storage. **BREAKING** (internal event payload shape; no external
  consumers).
- A new subscriber reacts to `PlaylistDeleted` by scheduling a new
  `DeletePlaylistFiles` task, matching the existing event -> subscriber ->
  task pattern (`ReconcileOnPlaylistCreated`, `DeleteVideoFileOnVideoDeleted`).
  The task recursively removes the playlist's output directory. This is
  safe to do unconditionally (no per-file filename matching needed) because
  path uniqueness now guarantees nothing else lives at that path.
- Update the SPA's delete-confirmation copy, which currently promises this
  won't happen automatically.

## Capabilities

### New Capabilities

(none — this extends existing capabilities)

### Modified Capabilities

- `playlist-crud`: adds a path-uniqueness requirement on playlist creation;
  adds a requirement that deleting a playlist removes all of its video
  records as part of the same operation.
- `video-cleanup`: adds a requirement that a playlist's output directory is
  removed from disk once the playlist is deleted.

## Impact

- `src/domain/playlist/service.rs` (`PlaylistService::create_playlist`,
  `create_custom_playlist`, `delete_playlist`)
- `src/domain/playlist/errors.rs` (new validation error variant)
- `src/domain/event/domain_event.rs` (`PlaylistDeleted` payload)
- `src/domain/video/service.rs` / `src/domain/task/task.rs` (new task type
  and handler)
- `src/infrastructure/repositories/sqlite_playlist_repository.rs` (path
  lookup for uniqueness check)
- `src/infrastructure/repositories/sqlite_video_repository.rs` (bulk delete
  for a playlist)
- `src/infrastructure/repositories/filesystem_video_file_repository.rs` (or
  a new port) for recursive directory removal
- `src/subscribers/mod.rs`, `src/tasks/mod.rs` (new registry entries)
- `web/src/components/PlaylistActionsMenu.jsx` (confirmation copy)
