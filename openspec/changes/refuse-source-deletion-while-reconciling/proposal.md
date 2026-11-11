# Proposal

## Why

Deleting a playlist or channel doesn't wait for, or stop, a sync (reconcile) of that source that is already running. A sync that is listing YouTube when the delete completes goes on to insert video rows for the deleted source, publish `VideoAddedToPlaylist`/`VideoAddedToChannel` and schedule `sync_video` tasks. Nothing ever cleans those rows up, which breaks playlist-crud's "no video record can ever be observed referencing a playlist that no longer exists". The bug predates this change; `/audit-change` found it and verification confirmed it. Likely trigger: adding a source and deleting it straight away, during its first sync.

## What Changes

- Deleting a playlist or channel while a sync of that same source is running is refused with 409 Conflict and the message "<playlist|channel> <id> is being synced, try again once it finishes". Nothing is deleted. This matches how moving a show already refuses while it syncs.
- A sync that starts after a delete still finds the source gone and does nothing (existing behaviour, unchanged).
- The web UI needs no code change: the delete confirmation dialog already shows the server's error message.
- The e2e `cleanLibrary` fixture retries a delete refused with 409 until it succeeds, so test cleanup doesn't flake when a sync is running.
- Out of scope: a sync that starts after the 409 and before the user retries (it just makes the retry refuse again); file-move/delete tasks.

## Capabilities

### New Capabilities

### Modified Capabilities
- `playlist-crud`: "Delete a Playlist" is refused while the playlist is being synced.
- `channel-crud`: "Delete Channel" is refused while the channel is being synced.

## Impact

- `src/domain/playlist/errors.rs`, `src/domain/channel/errors.rs`: new `Reconciling` variants.
- `src/domain/services/playlist_deleter.rs`, `channel_deleter.rs`: take `ShowLocks`, `try_lock` the source first.
- `src/application/http/playlists/mod.rs`, `channels/mod.rs`: map `Reconciling` to 409; every test constructing a deleter passes `ShowLocks`.
- `src/serve.rs`: wires the shared `ShowLocks` into both deleters.
- `e2e-tests/helpers/fixtures.js`: retry on 409.
- Both delta specs build on `cancel-tasks-of-deleted-videos`' version of the same requirements: archive that change first.
