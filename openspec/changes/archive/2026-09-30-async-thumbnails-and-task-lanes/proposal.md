## Why

After a deploy, several playlists were re-added at once. No downloads were queued for hours: the tasks tab stayed empty while thumbnails trickled in. The event consumer processes events one at a time. `PlaylistCreated` runs the whole reconcile inside its handler, and that reconcile fetched each new video's thumbnail inline by spawning `yt-dlp`, which took about 17s per video. Every `VideoAddedToPlaylist` event, and so every `DownloadVideo` task, queued behind that. The task executor is also strictly sequential, so even once downloads are queued, a large playlist's backlog drains one video at a time. Making thumbnails asynchronous and letting downloads run in parallel requires fixing several races that already exist today, because the event consumer and the task executor already run concurrently.

## What Changes

- **Thumbnails become tasks.** A new `FetchThumbnail` task type fetches one video's thumbnail. New subscribers on `VideoAddedToPlaylist` and `VideoAddedToChannel` schedule it, the same way downloads are scheduled today. Reconcilers no longer fetch thumbnails inline, so a reconcile pass only does API and DB work and publishes its events quickly.
- **Missing-thumbnail recovery schedules tasks.** A reconcile pass schedules a `FetchThumbnail` task for each video still missing a thumbnail, instead of fetching inline. Videos that already have a pending or running `FetchThumbnail` task are skipped.
- **Parallel task execution through lanes.** The executor runs tasks in lanes:
  - a **download lane** for `DownloadVideo`, with configurable concurrency (new `YARRTUBE_DOWNLOAD_CONCURRENCY`, default 2),
  - a **thumbnail lane** for `FetchThumbnail`, with a fixed concurrency of 1, so a thumbnail backlog never delays reconciles or deletes,
  - a **light lane** (concurrency 1) for reconciles and file deletions,
  - `UpdateYtdlp`, which runs **exclusively**: no other task runs while it does, and no new task starts while it waits.
  Tasks are claimed atomically, so no task is dispatched twice. Two tasks that touch the same video (download or thumbnail fetch) never run at the same time.
- **Duplicate tasks are not queued.** Scheduling a `DownloadVideo` or `FetchThumbnail` task for a video that already has one pending or running is a no-op.
- **Concurrency hardening (fixes latent bugs):**
  - The filesystem orphan sweep no longer deletes the folder of a video whose download is in progress or being retried but hasn't recorded a thumbnail yet.
  - The reconcile's title refresh and the thumbnail fetcher's thumbnail recording update only their own field, so they can no longer overwrite a concurrent status change made by a download.
  - Two concurrent downloads of different videos with the same title in the same directory no longer resolve to the same folder.
  - A download that finishes after its video (or its whole playlist/channel) was deleted removes the folder it wrote, instead of leaving a stray folder behind the directory cleanup.
- **Tasks tab** shows the new `fetch_thumbnail` task with the video's title and its playlist or channel.
- **README** documents `YARRTUBE_DOWNLOAD_CONCURRENCY`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `task-scheduling`: dispatch changes from one-at-a-time to lanes (download, thumbnail, light) with per-lane concurrency and an exclusive task type, plus atomic claim, per-video mutual exclusion, and dedupe of per-video tasks at schedule time. Ordering guarantees now apply within a lane.
- `video-thumbnails`: the fetch ahead of download happens asynchronously as a scheduled task, triggered by the video-added events, instead of inline during video creation.
- `playlist-reconciliation`: new-video persistence no longer fetches thumbnails inline. Missing-thumbnail recovery schedules deduplicated tasks. The orphan sweep also protects the folders of videos with downloads in flight.
- `channel-video-sync`: the same three changes as `playlist-reconciliation`, for channels.
- `video-download`: two concurrent downloads never share a folder, and a download that finishes for a deleted video leaves nothing behind on disk.
- `task-listing`: `fetch_thumbnail` tasks are listed with their video title and playlist or channel name.
- `web-ui`: the tasks tab describes `fetch_thumbnail` tasks.

## Impact

- **Domain:**
  - `Task` enum: new `FetchThumbnail` variant, plus a lane and an optional per-video exclusivity key per task.
  - `ThumbnailFetcher`, both reconcilers (`playlist_video_reconciler.rs`, `channel_video_reconciler.rs`), `VideoDownloader`.
  - `TaskViewSearcher`.
- **Application:**
  - new `fetch_thumbnail_task.rs`,
  - new `fetch_thumbnail_on_video_added_to_playlist.rs` and `fetch_thumbnail_on_video_added_to_channel.rs`,
  - registry wiring in `subscribers/mod.rs` and `tasks/mod.rs`.
- **Infrastructure:**
  - `TaskExecutor`: four lanes (download, thumbnail, light, exclusive) and per-video exclusivity.
  - `SqliteTaskRepository`: atomic claim with `UPDATE … RETURNING`, dedupe on schedule.
  - `SqliteVideoRepository`: targeted `update_title` and `update_thumbnail`.
  - `ytdlp.rs`: atomic creation of the video's folder.
  - No schema migration expected. The bundled SQLite supports `RETURNING` and `json_extract`.
- **Composition:** `serve.rs` reads `YARRTUBE_DOWNLOAD_CONCURRENCY`.
- **Frontend:** `web/src/components/TasksView.jsx`.
- **Operational:** more concurrent `yt-dlp` processes against YouTube. The default is kept low (2) to limit throttling risk.
