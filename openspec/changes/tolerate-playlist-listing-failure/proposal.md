## Why

When listing a playlist's items from the YouTube Data API fails, the reconcile
pass aborts. A playlist deleted or made private on YouTube, a long API outage,
an exhausted quota, or an expired API key therefore makes every pass fail. The
failure skips the filesystem steps and the next-reconcile scheduling. After 5
failed attempts (about 5h of backoff) the `ReconcilePlaylist` task is
dead-lettered, and that playlist is never reconciled again, silently, even once
the API recovers. Channels already tolerate the equivalent failure
(`channel-video-sync`'s "yt-dlp fails to list a channel's videos"); playlists
should behave the same way.

## What Changes

- A failure to list a playlist's current items is no longer fatal to the
  reconcile pass. The membership diff is skipped for that pass: no video is
  added, refreshed or removed, and no membership event is published. The
  failure is logged.
- The rest of the pass still runs from the stored videos: missing or non-mp4
  file healing, missing metadata, errored-video recovery, stranded-download
  recovery, missing thumbnails and orphan cleanup.
- A recurring pass still schedules the next reconcile, so the playlist keeps
  being reconciled and converges again once listing succeeds.
- An on-demand reconcile whose listing fails now responds successfully
  (it previously responded with an internal error), the same as for channels.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `playlist-reconciliation`: adds the requirement that a failed playlist
  listing leaves membership untouched while the filesystem steps and recurring
  scheduling still happen.

## Impact

- `src/domain/services/playlist_video_reconciler.rs`: listing errors are logged
  and turned into "skip membership" instead of being propagated.
- `src/infrastructure/repositories/youtube_playlist_items_repository.rs`: the
  test fake gains a failing constructor.
- `src/application/tasks/reconcile_playlist_task.rs` and
  `src/application/http/playlists/mod.rs`: new acceptance tests.
- No API, schema or configuration change.
