## Why

Yarrtube already discovers new playlist videos (`VideoAdded` is published by
`VideoService::sync_playlist_videos`, and stored as status `PENDING`), but
nothing ever consumes that event — a tracked playlist's videos are recorded
and then never actually downloaded outside the manual, one-shot CLI command.
This closes that loop: a video being added to a tracked playlist should
result in it being downloaded automatically, using the same
event → task → handler machinery already proven by `playlist-sync`.

## What Changes

- New subscriber `DownloadVideoOnVideoAdded` reacts to `VideoAdded` by
  scheduling a `Task::DownloadVideo{playlist_id, video_id}` (run now) rather
  than downloading inline — downloading is slow and external, so it belongs
  on the task queue to get retry/backoff/dead-letter for free.
- New `DownloadVideoTask` handler drives a new `VideoService.download_video`
  operation: transition the video to in-progress, shell out to `yt-dlp` for
  that single video, then transition it to downloaded or errored based on
  the outcome.
- `VideoStatus` gains `InProgress`, `Downloaded`, `ErroredRetrying`, and
  `Errored` (today only `Pending` exists). `ErroredRetrying` means a download
  attempt failed but the task queue still has retries left for it;
  `Errored` means the task was dead-lettered (retries exhausted). No error
  message is persisted on the video — status only.
- Every status transition is a method on the `Video` entity
  (`start_download`, `mark_downloaded`, `mark_errored_retrying`,
  `mark_errored`), consistent with `refactor-task-event-repositories`'s
  CRUD-repository/entity-transition pattern. `VideoRepository` gains plain
  `find`/`update`; the existing `upsert` (sync's insert-or-merge-title,
  status-preserving) is untouched.
- `TaskHandler::handle` (and `PersistedTask`/`ScheduledTask` from
  `refactor-task-event-repositories`) is widened so a handler can tell
  "this attempt still has retries left" from "this is the last attempt" —
  needed to decide `ErroredRetrying` vs `Errored`. `SyncPlaylistTask` ignores
  the new information; behavior for existing task types is unchanged.
- New per-video output directory: `<YARRTUBE_VIDEOS_PATH>/<playlist name>/`,
  looked up via the playlist's already filesystem-validated name.
  `YARRTUBE_VIDEOS_PATH` is a new environment variable, default `/videos`
  (matching the README's existing Docker volume mount point).
- The single-video `yt-dlp` invocation currently private to the CLI's
  `YoutubeDownloaderClient` (`infrastructure/client/`) is extracted into
  `infrastructure/shared/` so both the CLI client and a new
  domain-injected download port (`infrastructure/repositories/`) can shell
  out to `yt-dlp` for one video without duplicating that logic.
- README's configuration table gains `YARRTUBE_VIDEOS_PATH`.

**BREAKING**: none — no HTTP/CLI surface changes; `TaskHandler` is an
internal port with no external implementations.

## Capabilities

### New Capabilities
- `video-download`: automatically downloads a video via `yt-dlp` when it is
  added to a tracked playlist, tracking the video through
  pending → in-progress → downloaded, or in-progress → errored (retrying) →
  errored (permanently) on failure.

### Modified Capabilities
(none — `playlist-sync`'s "New Video Persistence" requirement, that a new
video is stored as `PENDING`, is unchanged; what happens to a `PENDING`
video next is entirely new behavior, captured in the new capability above.)

## Impact

- Depends on `refactor-task-event-repositories` landing first: this change
  builds `VideoRepository`'s CRUD shape and the widened `TaskHandler` on top
  of that change's `ScheduledTask`/CRUD `TaskRepository`, rather than adding
  a fifth `mark_*` method to the pre-refactor repository.
- `src/domain/video/`: `VideoStatus` gains variants; `Video` gains
  transition methods; `VideoService` gains `download_video`.
- `src/infrastructure/repositories/`: `VideoRepository` gains `find`/
  `update`; new download port (e.g. `youtube_video_downloader_repository.rs`)
  injected into `VideoService`.
- `src/infrastructure/shared/`: new module for the extracted single-video
  `yt-dlp` invocation.
- `src/infrastructure/client/youtube_downloader_client.rs`: `download_all`
  calls the extracted shared function instead of its own private one.
- `src/infrastructure/repositories/task_handler.rs`,
  `task_executor.rs`: `TaskHandler::handle` signature widened.
- `src/tasks/`: new `download_video_task.rs`, registered in `tasks::registry`.
- `src/subscribers/`: new `download_video_on_video_added.rs`, registered in
  `subscribers::registry`.
- `src/serve.rs`: reads `YARRTUBE_VIDEOS_PATH`, wires the new port and env
  value into `VideoService`.
- `README.md`: configuration table entry for `YARRTUBE_VIDEOS_PATH`.
- No HTTP API changes, no new CLI commands.
