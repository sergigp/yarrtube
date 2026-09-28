## Why

On the NAS, every video of the "bob el manetes" playlist fails with `ERROR: [youtube] <id>: This video is not available`, and the failures never stop. Two things combine:

- The Docker image ships no JavaScript runtime. Current yt-dlp needs one (Deno by default) to solve YouTube's player challenges; without it yt-dlp falls back to clients YouTube answers with `UNPLAYABLE` for some videos (made-for-kids ones among them). The same yt-dlp version downloads those videos fine on a machine with Deno installed.
- Reconcile resets every permanently errored video back to PENDING on *every* pass (hourly), with no limit. A video that can never download therefore cycles forever: 5 attempts with backoff, dead-letter, reset, repeat.

Because the task worker is serial and picks due tasks by ascending id, those ~75 old, always-failing tasks run ahead of any newer work each time they come due, delaying fresh downloads (e.g. for newly added channels) by several minutes per wave.

## What Changes

- Bundle Deno into the runtime Docker image so yt-dlp has a JavaScript runtime available on `PATH`.
- Playlist and channel reconcile only recover a permanently errored video (reset to PENDING + schedule a fresh download) once it has stayed permanently errored for at least 24 hours, instead of on every pass. Videos errored more recently are left untouched by that pass.
- The task executor dispatches eligible tasks in order of their scheduled run time (oldest due first, id as tiebreaker) instead of by id, so a long-lived retrying task no longer jumps ahead of work that became due earlier.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `container-image`: the image must also bundle a JavaScript runtime usable by yt-dlp.
- `playlist-reconciliation`: permanently failed video recovery gains a 24h cooldown instead of running every pass.
- `channel-video-sync`: same cooldown for permanently failed video recovery on channels.
- `task-scheduling`: eligible tasks are dispatched in order of scheduled run time.

## Impact

- `Dockerfile` (runtime stage: copy the `deno` binary from `denoland/deno:bin`).
- `src/domain/video/video.rs` (new `last_errored_at` field and a recovery-due predicate), `src/domain/services/playlist_video_reconciler.rs`, `src/domain/services/channel_video_reconciler.rs` (errored-video recovery filter).
- `src/infrastructure/repositories/sqlite_video_repository.rs` (row mapping) and a new migration adding a nullable `videos.last_errored_at` column.
- `src/infrastructure/repositories/sqlite_task_repository.rs` (`list_eligible` ordering).
- No HTTP API change. Image grows by the Deno binary (~100 MB uncompressed).
- Already-queued failing tasks on a running deployment are not touched by this change; they resolve on their own once the new image (with Deno) is running.
