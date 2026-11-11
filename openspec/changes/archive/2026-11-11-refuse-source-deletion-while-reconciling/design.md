# Design

## Files

- `src/domain/playlist/errors.rs`: `DeletePlaylistError::Reconciling`.
- `src/domain/channel/errors.rs`: `DeleteChannelError::Reconciling`.
- `src/domain/services/playlist_deleter.rs`: holds `ShowLocks`, `try_lock`s the playlist before anything else.
- `src/domain/services/channel_deleter.rs`: same for the channel.
- `src/application/http/playlists/mod.rs`: maps `Reconciling` to 409; tests pass `ShowLocks` to every `PlaylistDeleter::new`.
- `src/application/http/channels/mod.rs`: same for channels.
- `src/serve.rs`: passes the container's shared `show_locks` to both deleters.
- `e2e-tests/helpers/fixtures.js`: `deleteAllSources` retries a 409 delete until it succeeds.

## Types & Signatures

```rust
pub enum DeletePlaylistError {
    NotFound(PlaylistId),
    Reconciling(PlaylistId), // "playlist {id} is being synced, try again once it finishes"
    Repository(anyhow::Error),
}

pub enum DeleteChannelError {
    NotFound(ChannelHandle),
    Reconciling(ChannelHandle), // "channel {handle} is being synced, try again once it finishes"
    Repository(anyhow::Error),
}

impl PlaylistDeleter {
    pub fn new(/* ...existing params... */, show_locks: Arc<ShowLocks>) -> Self;
    fn lock(&self, id: &PlaylistId) -> Result<ShowLockGuard<'_>, DeletePlaylistError>;
}

impl ChannelDeleter {
    pub fn new(/* ...existing params... */, show_locks: Arc<ShowLocks>) -> Self;
    fn lock(&self, id: &ChannelHandle) -> Result<ShowLockGuard<'_>, DeleteChannelError>;
}
```

`lock` uses `ShowLocks::try_lock(ShowKey::Playlist(id))` / `ShowKey::Channel(id)`, like `ShowMover::lock`: refuse, never wait.

## Call Stack

Delete a playlist (`DELETE /api/playlists/{id}`):
```
delete_playlist handler (id)
  → PlaylistDeleter::delete(id)
    → let _guard = lock(&id)?          // new: try_lock(ShowKey::Playlist(id)) or Err(Reconciling(id))
    → find_playlist(&id)
    → delete_playlist_videos(&id)
    → remove_pending_reconciles(&id)
    → delete_playlist(&id)
    → publish_playlist_deleted(&id, &playlist)
  ← Err(Reconciling) → ApiError::new(StatusCode::CONFLICT, e)
```

Delete a channel: same through `ChannelDeleter::delete` with `ShowKey::Channel(id)`; `delete_channel` maps `Reconciling` to 409.

A reconcile that starts after the delete already takes the lock and then `find_playlist`/`find_channel`s, gets `None` and returns (unchanged).

## Test Plan

1. Behaviour (HTTP handler tests, real SQLite; the test holds `show_locks.lock(..)` itself to stand in for a running reconcile)
   1. `it_should_fail_to_delete_a_playlist_while_it_is_being_synced`: 409 with the exact message; the playlist, its video and its pending task are still stored; no event in the outbox.
   2. `it_should_delete_a_playlist_while_another_source_is_being_synced`: a lock held on another playlist doesn't block it: 204, playlist gone.
   3. `it_should_fail_to_delete_a_channel_while_it_is_being_synced`: as 1.1 for a channel.
   4. `it_should_delete_a_channel_while_another_source_is_being_synced`: as 1.2 for a channel.
2. Infrastructure: none (`ShowLocks` is unchanged and already tested).
