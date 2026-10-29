# Design

## Files

- `src/domain/services/video_reconciler.rs` — owns one video's whole lifecycle; trait reduced to `reconcile`; gains reset, recovery, metadata repair (moved from `internal_video_reconciler.rs`) and the stranded warning.
- `src/domain/services/internal_video_reconciler.rs` — reduced to: sync every stored video, then sweep orphaned files from a fresh snapshot. `MembershipDelta`, `sort_positions`, `handled`, broken/recovery/metadata code removed.
- `src/domain/services/channel_video_reconciler.rs`, `playlist_video_reconciler.rs` — no `assign_categories`, no delta passed down, smaller `DesiredState`.
- `src/domain/services/video_category_assigner.rs` — `assign_source` removed (no caller left).
- `src/infrastructure/shared/video_lock.rs` — new in-process per-video lock (sibling of `category_lock.rs`).
- `src/infrastructure/repositories/sqlite_task_repository.rs` — new read for the stranded check.
- `src/serve.rs` — `VideoLock` in `InfrastructureContainer` (one instance shared by every `VideoReconciler`), new `VideoReconciler` dependencies, slimmer `InternalVideoReconciler`.
- `src/application/tasks/sync_video_task.rs` — primary acceptance suite of the lifecycle (tests moved here).
- `src/application/tasks/reconcile_channel_task.rs`, `reconcile_playlist_task.rs` — per-video recovery tests removed or rewritten as pass-wiring tests.

## Types & Signatures

```rust
// video_lock.rs
#[derive(Default)]
pub struct VideoLock {
    held: Mutex<HashSet<VideoRecordId>>,
    released: Condvar,
}
pub struct VideoLockGuard<'a> { lock: &'a VideoLock, video_id: VideoRecordId }
impl VideoLock {
    /// Locks `video_id`, waiting while another holder has it; released on drop.
    pub fn lock(&self, video_id: &VideoRecordId) -> VideoLockGuard<'_>;
}
impl Drop for VideoLockGuard<'_> { fn drop(&mut self); }

// video_reconciler.rs
struct VideoOwner {
    source: CategoryMember,
    target: VideoTarget,
    sort_position: Option<i64>, // playlist position; None for a channel
}

pub struct VideoReconciler {
    video_repository: Arc<dyn VideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    task_repository: Arc<dyn TaskRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,         // new
    video_metadata_repository: Arc<dyn VideoMetadataRepository>, // new
    metadata_generator: Arc<MetadataGenerator>,                   // new
    video_relocator: Arc<VideoRelocator>,                         // new
    video_lock: Arc<VideoLock>,                                   // new
    clock: Arc<dyn Clock>,
    videos_path: String,
    video_category_assigner: VideoCategoryAssigner,
}

pub trait VideoReconcilerApi: Send + Sync {
    /// Brings the stored video to its desired state, holding its lock:
    /// categories, broken-file reset, errored recovery, metadata repair,
    /// stranded warning, then schedules every step still needed. No-op if
    /// the video or its owner is gone. Idempotent.
    fn reconcile(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
}
// removed: reconcile_video, schedule_download

impl VideoReconciler { // private helpers
    fn find_owner(&self, video: &Video) -> anyhow::Result<Option<VideoOwner>>;
    fn reset_if_broken(&self, video: Video, target: &VideoTarget) -> anyhow::Result<Video>;
    fn recover_if_due(&self, video: Video) -> anyhow::Result<Video>;
    fn repair_metadata(&self, video: &Video, owner: &VideoOwner); // best effort, logs
    fn warn_if_stranded(&self, video: &Video) -> anyhow::Result<()>;
    fn schedule_steps(&self, video: &Video, target: &VideoTarget) -> anyhow::Result<()>;
}

// internal_video_reconciler.rs
pub struct DesiredState {
    pub path: PlaylistPath,
    pub video_ids: Vec<VideoRecordId>,
} // removed: quality, sort_positions; MembershipDelta removed

pub struct InternalVideoReconciler {
    video_repository: Arc<dyn VideoRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    video_reconciler: VideoReconciler,
    videos_path: String,
}

pub trait InternalVideoReconcilerApi: Send + Sync {
    /// Reconciles every video of `desired` (`VideoReconciler::reconcile`,
    /// a failing video is logged and skipped), then deletes orphaned files
    /// from a snapshot read after those reconciles.
    fn reconcile(&self, desired: &DesiredState) -> anyhow::Result<()>;
}

// sqlite_task_repository.rs (TaskRepository)
/// The pending or running task of `task_type` for `video_id`, if any.
fn find_non_completed_for_video(
    &self,
    task_type: &str,
    video_id: &VideoRecordId,
) -> anyhow::Result<Option<ScheduledTask>>;

// video_category_assigner.rs — removed from VideoCategoryAssignerApi:
// fn assign_source(&self, source: &CategoryMember) -> anyhow::Result<()>;
```

## Call Stack

Video sync (event-triggered):
```
SyncVideoTask::handle(payload)
  VideoReconciler::reconcile(&video_id)
    video_lock.lock(&video_id)
    video_repository.find(&video_id)                       -> None: Ok
    find_owner(&video)                                     -> None: Ok
      channel_video_repository.find_by_video -> channel_repository.find
      | playlist_video_repository.find_by_video (position) -> playlist_repository.find
    video_category_assigner.assign_video(&video.id, &owner.source)
    reset_if_broken(video, &owner.target)                  // Downloaded && !healthy file
      video.reset_for_redownload(now) -> video_repository.update(&reset)
    recover_if_due(video)                                  // video.is_due_for_recovery(now)
      video.reset_for_redownload(now) -> video_repository.update(&reset)
    repair_metadata(&video, &owner)                        // Downloaded && no metadata row
      metadata_generator.generate(&video, owner.sort_position, None)
      video_relocator.relocate(..)                         // legacy only, unchanged
      video_metadata_repository.save(&id, &metadata, &number, &nfo_path)
    warn_if_stranded(&video)                               // !settled && no download task
      task_repository.find_non_completed_for_video("download_video", &video.id)
    schedule_steps(&video, &owner.target)
      VideoSyncPlan::for_video(&video).steps -> task_repository.schedule(..)
```

Reconcile pass (channel; playlist identical):
```
ReconcileChannelTask::handle(payload)
  ChannelVideoReconciler::reconcile(&channel_id)
    sync membership with YouTube (unchanged)
    InternalVideoReconciler::reconcile(&DesiredState { path, video_ids })
      for each id: VideoReconciler::reconcile(&id)          // inline, sequential; Err -> warn!, continue
      read_actual_state(&desired)                          // files listed, then videos read
      delete_orphaned_files(&actual)
    schedule_next_reconcile(&channel_id) (unchanged)
```

## Test Plan

Behaviour (acceptance) — `sync_video_task.rs`, primary suite of the lifecycle:
1. `it_should_reset_a_video_whose_file_is_missing` — Downloaded video, no file: stored video equals `reset_for_redownload(now)` (number kept, subtitles/sync time cleared); thumbnail fetch + download scheduled.
2. `it_should_reset_a_video_whose_file_is_not_mp4` — same for a `.mkv` file.
3. `it_should_recover_an_errored_video_after_its_cooldown` — Errored ≥24h: reset, thumbnail fetch + download scheduled (no recovery counter, so it also covers a video errored again after a recovery).
4. `it_should_schedule_nothing_for_a_settled_errored_video` (existing, regression) — Errored <24h: unchanged, nothing scheduled.
5. `it_should_never_recover_an_excluded_video` — Excluded long ago: unchanged, nothing scheduled.
6. `it_should_generate_missing_metadata_of_a_playlist_video` — Downloaded, no metadata row: metadata saved with sorttitle from playlist position.
7. `it_should_generate_missing_metadata_of_a_channel_video` — sorttitle from publish date.
8. `it_should_keep_existing_metadata` — metadata row present: generator not called.
9. `it_should_relocate_a_legacy_video_when_its_metadata_is_recovered` — legacy file moved into season folder, filenames recorded.
10. `it_should_leave_a_legacy_video_in_place_when_its_metadata_is_still_unavailable` — generator returns none: video unchanged.
11. `it_should_warn_about_a_stranded_video` — Pending, no download task: warning logged, download scheduled.
12. `it_should_keep_a_stranded_errored_retrying_video_errored_retrying` — warning logged, download scheduled, status unchanged.
13. `it_should_not_warn_about_a_video_whose_download_is_running` — running download task: no warning, no second download.

Behaviour (acceptance) — `reconcile_channel_task.rs`:
14. `it_should_redownload_videos_with_missing_file` (update) — pass resets the video; thumbnail fetch + download scheduled (replaces `it_should_schedule_only_the_download_of_a_video_reset_for_redownload`).
15. `it_should_sync_a_video_added_by_the_same_pass` (replaces `it_should_schedule_no_step_for_a_video_added_by_the_same_pass`) — new video's steps scheduled once, no duplicates.
16. `it_should_repair_the_categories_of_stored_videos` (update) — one video-categories-changed event per repaired video.
17. `it_should_delete_the_stale_file_of_a_video_reset_by_the_same_pass` — `.mkv` of a reset video swept in the same pass.

Behaviour (acceptance) — `reconcile_playlist_task.rs`:
18. `it_should_redownload_videos_with_missing_file` (update) — as 14.
19. `it_should_sync_a_video_added_by_the_same_pass` (replaces `it_should_not_schedule_a_thumbnail_fetch_for_a_video_added_in_the_same_pass`) — as 15.
20. `it_should_repair_the_categories_of_stored_playlist_videos` (update) — as 16.

Removed (covered by 1–13): per-video tests in `internal_video_reconciler.rs` (redownload, retry, reschedule, metadata, thumbnail-fetch ones) and the pass-level recovery duplicates in both task files; orphan-sweep tests stay in `internal_video_reconciler.rs`.

Not unit tested: "one video fails to sync" (every fallible step of a sync hits real SQLite, so no deterministic trigger) — guaranteed by the `warn!`-and-continue in `InternalVideoReconciler::reconcile`; "concurrent syncs serialized" end to end — covered by the `VideoLock` tests below.

Infrastructure — `video_lock.rs`:
21. `it_should_block_a_second_lock_of_the_same_video_until_released`
22. `it_should_not_block_locks_of_different_videos`

Infrastructure — `sqlite_task_repository.rs`:
23. `it_should_find_a_running_task_of_a_video`
24. `it_should_not_find_a_completed_task_of_a_video`
