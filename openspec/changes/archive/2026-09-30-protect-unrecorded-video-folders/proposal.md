## Why

The audit of `async-thumbnails-and-task-lanes` found two ways the orphan sweep can delete a folder a `yt-dlp` process is still writing into, and one way a task can get stuck `running` and block its video's later tasks until a restart. With thumbnail fetches and downloads now running in parallel with reconciles, all three happen in normal use, not only in rare races. The fixes are already implemented on `bug-download-videps` (commits `148ebdf`..`9c1a8d9`); this change records the rules they introduce, since the synced main specs and the archived design (D5) still describe the narrower behaviour.

## What Changes

- **Orphan sweep protects every unrecorded video folder.** A reconcile pass protects the predicted folder names (the sanitized title, and `"{title} [{youtube_id}]"`) of every stored video whose download folder is not recorded yet, whatever its status. Before, only In Progress and errored-but-retrying videos were protected, so a Pending video's folder was deleted while its thumbnail fetch was still writing it (the fetch records the folder only when it finishes).
- **Renames keep the in-flight folder.** When a reconcile pass changes a stored video's title and that video's folder is not recorded yet, the pass also protects the folders predicted from the previous title, since a download started before the rename is still writing under the old name. The pass writes a video's title only when it actually changed.
- **Stuck running tasks are recovered without a restart.** A task marked `running` that the executor is not actually running (for example, recording its outcome failed) is treated as a failed attempt on the next scheduling pass, the same way crash recovery treats one at startup. Before, it stayed `running` until the daemon restarted, and the no-duplicate rule silently dropped every later download or thumbnail fetch for its video.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `playlist-reconciliation`: "Filesystem Reconciliation Against Recorded Downloads" protects the predicted folders of every video whose folder is not recorded yet, and of a renamed video's previous title.
- `channel-video-sync`: the same change to "Filesystem Reconciliation Against Recorded Downloads (Channels)".
- `task-scheduling`: "Crash Recovery" also recovers, during normal operation, a task left `running` with no attempt in progress.

## Impact

- **Domain:** `Video::unrecorded_folder_candidates` (replaces `in_flight_download_folders`); both reconcilers' membership sync returns the added ids and each renamed video's previous title, and writes a title only when it changed.
- **Infrastructure:** `TaskExecutor` tracks the ids it is running and recovers other `running` rows on every scheduling pass; startup recovery shares that code.
- **Tests:** new acceptance tests for both reconcilers and the executor, plus wiring tests for the task-handler and event-subscriber registries and the executor's run loop.
- No schema migration, no API or UI change.
- **Trade-off:** a genuinely stale folder whose name matches a not-yet-downloaded video now survives until that video's download records its real folder (previously only In Progress videos kept such a folder alive).
