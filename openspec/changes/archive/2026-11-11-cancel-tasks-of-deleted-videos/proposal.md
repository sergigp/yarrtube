# Proposal

## Why

Deleting a playlist or channel deletes its videos but leaves their pending per-video tasks queued (e.g. a `download_video` re-queued 10 minutes after a failure). They stay listed in `GET /api/tasks` until they run and settle as no-ops against a video that no longer exists. Found via e2e: `erroredDownload.spec.js` leaves such a task behind, and `playlist.spec.js` "sync changes nothing when up to date", run next on the same worker, sees it.

## What Changes

- Deleting a playlist or channel also removes the pending per-video tasks (`download_video`, `fetch_thumbnail`, `fetch_subtitles`, `fetch_chapters`, `sync_video`, `upgrade_quality`, `summarize_video`, `categorize_video`) of every video it deletes, as part of the same operation, like the pending reconcile is removed today.
- Running tasks are left alone: they already settle without error when their video is gone.
- Tasks of other playlists' and channels' videos, and the `delete_*_files` tasks the deletion schedules, are unchanged.
- Out of scope: videos removed one at a time during a reconcile (no longer in the playlist / beyond the channel's limit) keep today's behaviour.

## Capabilities

### New Capabilities

### Modified Capabilities
- `playlist-crud`: "Delete a Playlist" also removes the pending tasks of the playlist's videos.
- `channel-crud`: "Delete Channel" also removes the pending tasks of the channel's videos.

## Impact

- `src/infrastructure/repositories/sqlite_task_repository.rs`: new `TaskRepository` method deleting a video's pending tasks.
- `src/domain/services/playlist_deleter.rs`, `src/domain/services/channel_deleter.rs`: call it for each deleted video.
- Tests in `src/application/http/playlists/mod.rs`, `src/application/http/channels/mod.rs`, and the task repository.
- No API, schema or UI change. Fixes the e2e test-order dependency without touching the e2e specs.
