## Why

When a playlist sync finds that a previously-tracked video is no longer on
YouTube, it deletes the stored video record (`playlist-sync`'s "Removed
Video Cleanup" requirement) but never touches the file `yt-dlp` may have
already downloaded for it. Those files accumulate on disk forever with
nothing left in the database to point to them. We need sync's removal to
also publish a `VideoDeleted` domain event and react to it by deleting the
downloaded file, if one exists, the same way `VideoAdded` already drives
downloading via the domain-events/task-scheduling pipeline.

## What Changes

- Add `DomainEvent::VideoDeleted { playlist_id, video_id, title, was_downloaded }`,
  published by `VideoService::sync_playlist_videos` for every video it
  deletes, carrying that video's title (needed to locate its file — the
  actual on-disk filename isn't stored anywhere) and whether its status was
  `Downloaded` at the moment of removal.
- Add a `DeleteVideoFileOnVideoDeleted` subscriber that schedules a
  `DeleteVideoFile` task only when `was_downloaded` is true — a video that
  never finished downloading has no file to clean up.
- Add a `DeleteVideoFile` task/handler that looks up the video's playlist
  (to resolve its output directory), searches that directory for a file
  matching the video's sanitized title (accounting for the `[video_id]`
  collision suffix `yt-dlp` may have appended at download time — see
  `resolve_collision` in `src/infrastructure/shared/ytdlp.rs`), and deletes
  it if found. No-ops (without erroring) if the playlist no longer exists or
  no matching file is found, so the task is safe to retry.
- Accepted, deliberately unaddressed race: a video removed from its playlist
  while its download is still in flight can still leave an orphaned file
  (the deletion decision is made from the video's status *at sync time*,
  which won't yet reflect a download that finishes afterward). Given syncs
  run hourly and a future reconcile-style sync rewrite is planned, this
  narrow window is accepted rather than solved with a soft-delete/tombstone
  mechanism.

## Capabilities

### New Capabilities

- `video-cleanup`: deletes a video's downloaded file from disk once that
  video is no longer part of its playlist.

### Modified Capabilities

- `playlist-sync`: the "Removed Video Cleanup" requirement gains publishing
  a `VideoDeleted` event (with the removed video's title and whether it had
  been downloaded) alongside deleting its stored record.

## Impact

- `src/domain/event/domain_event.rs`: new `VideoDeleted` variant.
- `src/domain/video/service.rs`: `sync_playlist_videos` publishes
  `VideoDeleted` for each video it deletes.
- `src/domain/task/task.rs`: new `DeleteVideoFile` task variant.
- `src/subscribers/`: new `delete_video_file_on_video_deleted.rs`,
  registered in `subscribers::registry` for event type `video_deleted`.
- `src/tasks/`: new `delete_video_file_task.rs`, registered in
  `tasks::registry` for task type `delete_video_file`.
- `src/infrastructure/repositories/`: new port (trait + implementation) for
  locating and deleting a video's file on disk, injected into
  `VideoService`, mirroring how `VideoDownloaderRepository` is structured.
- `openspec/specs/playlist-sync/spec.md`: delta for the modified
  requirement.
- `openspec/specs/video-cleanup/spec.md`: new capability spec.
