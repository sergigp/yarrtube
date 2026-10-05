## Why

A failed YouTube listing is tolerated by every reconcile pass, including the
first one triggered by `PlaylistCreated` / `ChannelCreated`. A playlist or
channel created during a brief API or `yt-dlp` failure therefore stays empty
for a whole reconcile interval (15–60 min), because the creation event is
consumed as a success and never retried.

## What Changes

- The creation-triggered (initial) reconcile pass of a playlist or channel
  fails when listing its items/videos fails. Nothing is stored, no event is
  published, and no recurring reconcile is scheduled. The creation event's
  processing fails, so the events consumer retries it with backoff.
- If every retry of the creation event fails, the event is dead-lettered and
  the playlist or channel gets no recurring reconcile. This is accepted.
- Recurring and on-demand passes keep tolerating a listing failure, unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `playlist-reconciliation`: "Playlist Listing Failure Is Not Fatal" is scoped
  to recurring and on-demand passes; "Initial Reconcile After Playlist
  Creation" gains a listing-failure scenario.
- `channel-video-sync`: the "yt-dlp fails to list a channel's videos" scenario
  is scoped to recurring and on-demand passes; "Reconcile On Channel Creation"
  gains a listing-failure scenario.

## Impact

- `src/domain/services/playlist_video_reconciler.rs`,
  `src/domain/services/channel_video_reconciler.rs`: new `initial_reconcile`.
- `src/application/subscribers/reconcile_on_playlist_created.rs`,
  `src/application/subscribers/reconcile_on_channel_created.rs`: call it.
- No API, schema or configuration change.
- Already implemented in the working tree, together with unrelated audit
  fixes (reconciler read order, batched video reads, test asserts) that are
  not part of this change.
