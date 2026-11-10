## Why

A `videos` row is protected from concurrent writers by six overlapping
mechanisms (task exclusivity keys, `VideoLock`, column-specific repository
writes, merge-on-save of watch state, re-read guards, and plain
read-modify-write), none of which covers every writer. Synchronous HTTP
writes (watch progress, mark watched) do a whole-row write with no
protection at all, so they silently revert late data (subtitles, chapters,
quality, thumbnail) and can revert another copy's finished download. Every
new `Video` field also has to pick which partial-update method owns it,
which blocks breaking up the 27-field entity (idea 8).

## What Changes

- Add an optimistic `version` to every `videos` row. `VideoRepository::update`
  stores the whole entity only if the stored version still matches, else fails
  with a typed stale-video error.
- Add a `VideoUpdater` domain service: find → apply a pure transition →
  versioned update, retrying on the latest stored state when another writer
  stored first. Every mutation of an existing video goes through it, HTTP
  writes included (they stay synchronous).
- **Remove** the column-specific writes `update_title`, `update_subtitles`,
  `update_summary`, `update_thumbnail`, `update_chapters`, `update_quality`,
  and `Video::with_watch_state_of`. `save` (insert-or-replace) becomes
  `insert` (new rows only).
- The atomic writes that remain (`assign_episode_number`,
  `reset_unavailable_summaries`) bump the version.
- The reconcile pass (recurring task, `*Created` subscribers, HTTP
  force-reconcile) **schedules a `sync_video` task** per stored video instead
  of calling `VideoReconciler` inline, so all per-video work (reset,
  relocation, metadata repair) runs under the `video:{id}` task key.
- **Remove** `VideoLock`, which is now redundant: the key already serializes
  two syncs of one video.
- Responsibilities after the change: task keys guarantee that per-video *work*
  (yt-dlp, files, LLM calls) never overlaps; versions guarantee that no *row
  write* is lost.

## Capabilities

### New Capabilities
- `video-write-consistency`: concurrent updates of one stored video from different operations never lose each other's effects.

### Modified Capabilities
- `video-sync`: reconcile passes schedule each stored video's sync as a task instead of running it inline, and two syncs of one video are serialized by the task queue.

## Impact

- Migration `0026_video_version.sql` (`version INTEGER NOT NULL DEFAULT 0`).
- `Video` gains `version`. `VideoRepository` loses six methods, `save`
  becomes `insert`, and `update` becomes versioned.
- New `domain/services/video_updater.rs` and a `StaleVideo` error in
  `domain/video/errors.rs`.
- Callers move to `VideoUpdater`: downloader, quality upgrader, subtitle,
  chapter and thumbnail fetchers, summarizer, relocator, video reconciler,
  channel and playlist reconcilers (title refresh), and the watch-state updater.
- `InternalVideoReconciler` takes the `TaskRepository` instead of
  `VideoReconciler`. `infrastructure/shared/video_lock.rs` is deleted, along
  with its wiring in the container and its use in tests.
- Visible effect: after a reconcile pass or force-reconcile returns, the
  per-video steps (reset, recovery, step scheduling) are still queued as
  `sync_video` tasks instead of done. A stale file of a reset video is swept
  by the next pass, not the same pass.
- Test churn: reconcile-task tests that assert per-video sync effects inline
  now assert the scheduled `sync_video` tasks, and those effects stay covered
  by the `sync_video_task` tests. Test seeds rename `save` to `insert`.
