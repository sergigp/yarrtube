# Proposal

## Why

A video record's lifecycle (download, thumbnail, subtitles) is spread across
event subscribers (`DownloadVideoOn*`, `FetchThumbnailOn*`,
`FetchSubtitlesOnVideoDownloaded`) and the recovery passes of
`InternalVideoReconciler`, each deciding on its own which step to queue. The
next two changes add per-video steps (category assignment, Plex episode
labels), so the lifecycle needs one owner that decides what a video still
needs before more steps are bolted on in more places.

## What Changes

- New `VideoReconciler` domain service: for one video record, decides from
  its stored state which steps it still needs (download, thumbnail fetch,
  subtitle fetch) and schedules each as its existing task, in that step's
  existing lane. Every step is guarded by the video's state, so rerunning
  it only schedules the steps not done yet.
- New `sync_video` task (payload: the video record id) running the
  `VideoReconciler` for one video, in a new **video sync lane**
  (concurrency 1), so it is never held up by a long playlist/channel
  reconcile. It is shown in the tasks view's **Syncs** tab with the video's
  title.
- The `VideoAddedToPlaylist`, `VideoAddedToChannel` and `VideoDownloaded`
  events schedule a `sync_video` instead of a download, thumbnail fetch or
  subtitle fetch directly. The five subscribers doing that are replaced by
  three, one per event (`SyncVideoOnVideoAddedToPlaylist`,
  `SyncVideoOnVideoAddedToChannel`, `SyncVideoOnVideoDownloaded`).
- `InternalVideoReconciler` keeps its folder-level work (broken files,
  metadata repair, errored recovery, orphan cleanup) but delegates per-video
  step scheduling (stranded downloads, missing thumbnails, missing
  subtitles) to the `VideoReconciler`.
- YouTube calls stay sequential per video (per-video mutual exclusion is
  unchanged); download concurrency stays global and configurable.
- No change to when or how often downloads, thumbnails or subtitles happen,
  nor to the Plex scan triggers (`VideoDownloaded` and
  `VideoSubtitlesFetched` are still published and still trigger scans).

Future dependents: `categories-per-video` and `plex-category-labels` add
their steps to the `VideoReconciler` rather than new subscribers.

## Capabilities

### New Capabilities
- `video-sync`: the per-video sync: when it is scheduled, which steps it
  decides a video still needs, and that it never redoes a finished step.

### Modified Capabilities
- `task-scheduling`: a new video sync lane; video syncs take part in
  per-video mutual exclusion and are deduplicated against pending syncs
  only.
- `task-listing`: a video sync task resolves its video's title and
  playlist/channel name.
- `web-ui`: the tasks view describes video syncs and lists them in the
  Syncs tab.

## Impact

- Domain: new `VideoReconciler` service and `VideoSyncStep` /
  `VideoSyncPlan` / `VideoTarget` types; `InternalVideoReconciler`,
  `Task`, `TaskLane` change.
- Application: new `SyncVideoTask` and the three `SyncVideoOn*`
  subscribers; five
  subscribers removed; task and subscriber registries rewired.
- Infrastructure: `SqliteTaskRepository` dedupe and `TaskExecutor` lane
  capacity; `TaskViewSearcher` resolves `sync_video`.
- Web: `describeTask` / `taskFamily` in `web/src/lib/tasks.ts`.
- No schema migration, no new environment variable, no API shape change.
