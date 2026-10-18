## Why

Deleting a channel or playlist leaves its pending recurring reconcile queued, and nothing stops a second one being scheduled for the same source. When the same source is added again, its new reconcile chain runs alongside the stale one(s), and every chain schedules its own successor, so the source is reconciled several times per interval for good (the smoke suite showed four parallel syncs of one playlist). Changing a source's interval also only brings one of those pending reconciles forward.

## What Changes

- Deleting a channel or playlist removes its pending recurring reconcile, so no queued reconcile outlives the source.
- A channel or playlist has at most one pending recurring reconcile: when a reconcile pass (initial or recurring) finishes and one is already pending for that source, it schedules none.
- Deployments that already run several reconcile chains for a source converge to one on their own: each extra chain ends the next time it finishes while another is pending. No migration.

Builds on `per-source-reconcile-interval` (reuses `TaskRepository::find_pending` and the requirement texts it introduces); archive that change first.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `channel-crud`: Delete Channel also removes the channel's pending recurring reconcile.
- `playlist-crud`: Delete a Playlist also removes the playlist's pending recurring reconcile.
- `channel-video-sync`: Recurring Channel Reconciliation schedules no next reconcile when one is already pending for the channel.
- `playlist-reconciliation`: Recurring Reconciliation schedules no next reconcile when one is already pending for the playlist.

## Impact

- **Domain**: `ChannelDeleter` and `PlaylistDeleter` gain the task repository and remove the source's pending reconcile; `ChannelVideoReconciler` and `PlaylistVideoReconciler` check for a pending reconcile before scheduling.
- **Composition root**: `serve.rs` passes the task repository to both deleters.
- **No** schema, HTTP, web UI or config change.
