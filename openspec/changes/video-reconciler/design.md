## Files

- `src/domain/video/video_target.rs` — new `VideoTarget` VO: where and at which quality a video's owner wants it.
- `src/domain/video/video_sync_step.rs` — new `VideoSyncStep`: one lifecycle step and the task that performs it. **Extension point** for later changes.
- `src/domain/video/video_sync_plan.rs` — new `VideoSyncPlan`: the steps a video's stored state still needs (pure).
- `src/domain/video/mod.rs` — exports the three new types.
- `src/domain/services/video_reconciler.rs` — new `VideoReconciler`: resolves a video's owner and schedules its plan.
- `src/domain/services/mod.rs` — exports `VideoReconciler`.
- `src/domain/services/internal_video_reconciler.rs` — drops stranded/thumbnail/subtitle scheduling; delegates per-video steps to `VideoReconciler`.
- `src/domain/services/thumbnail_fetcher.rs` — removes `schedule_missing` and its `task_repository` dep (scheduling is now `VideoReconciler`'s job).
- `src/domain/services/subtitle_fetcher.rs` — removes `schedule_missing` and its `task_repository` dep.
- `src/domain/services/task_view_searcher.rs` — resolves a `sync_video` task's video title and owner name.
- `src/domain/task/task.rs` — `Task::SyncVideo`, its decode, lane, exclusivity key and running-dedupe rule.
- `src/domain/task/task_lane.rs` — `TaskLane::VideoSync` (capacity 1 via the executor's default).
- `src/application/tasks/sync_video_task.rs` — new `SyncVideoTask` handler.
- `src/application/tasks/mod.rs` — registers `sync_video`.
- `src/application/subscribers/sync_video_on_video_event.rs` — new subscriber scheduling a `sync_video` from any event carrying `video_id`.
- `src/application/subscribers/mod.rs` — registers it for `VideoAddedToPlaylist`, `VideoAddedToChannel`, `VideoDownloaded`; drops the five replaced subscribers.
- `src/application/subscribers/download_video_on_video_added_to_{channel,playlist}.rs`, `fetch_thumbnail_on_video_added_to_{channel,playlist}.rs`, `fetch_subtitles_on_video_downloaded.rs` — deleted.
- `src/infrastructure/repositories/sqlite_task_repository.rs` — dedupe checks `running` only when `Task::dedupes_running` says so.
- `src/serve.rs` — builds `VideoReconciler`, passes it to `InternalVideoReconciler`, the task registry and the subscriber registry.
- `web/src/lib/tasks.ts` (+ `tasks.test.ts`) — describes `sync_video`, family `syncs`.

## Types & Signatures

```rust
// domain/video/video_target.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoTarget {
    pub output_dir: PathBuf,
    pub quality: Quality,
}

// domain/video/video_sync_step.rs — later changes add variants here
// (e.g. AssignCategories, SyncPlexLabels) plus their rule in VideoSyncPlan
// and, for slow non-YouTube work, their own TaskLane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoSyncStep {
    Download,
    FetchThumbnail,
    FetchSubtitles,
}

impl VideoSyncStep {
    /// The task performing this step for `video`, run in that task's own lane.
    pub fn task_for(&self, video: &Video, target: &VideoTarget) -> Task;
}

// domain/video/video_sync_plan.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSyncPlan {
    pub steps: Vec<VideoSyncStep>,
}

impl VideoSyncPlan {
    /// Download if !is_download_settled; FetchThumbnail if no thumbnail,
    /// is_thumbnail_fetchable and not InProgress; FetchSubtitles if
    /// is_subtitle_fetchable. In-flight duplicates are left to TaskRepository dedupe.
    pub fn for_video(video: &Video) -> Self;
}

// domain/services/video_reconciler.rs
pub struct VideoReconciler {
    video_repository: Arc<dyn VideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

pub trait VideoReconcilerApi: Send + Sync {
    /// Finds the video and the playlist/channel holding it, then schedules
    /// its plan. No-op if either is gone.
    fn reconcile(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
    /// Schedules the plan of a video already read with its owner's target
    /// (a reconcile pass).
    fn reconcile_video(&self, video: &Video, target: &VideoTarget) -> anyhow::Result<()>;
    /// Schedules only the download, for a video a reconcile pass just reset.
    fn schedule_download(&self, video: &Video, target: &VideoTarget) -> anyhow::Result<()>;
}

// domain/services/internal_video_reconciler.rs — constructor change only
pub fn new(
    video_repository: Arc<dyn VideoRepository>,
    metadata_generator: Arc<MetadataGenerator>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    task_repository: Arc<dyn TaskRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    video_reconciler: VideoReconciler, // replaces subtitle_fetcher + thumbnail_fetcher
    video_relocator: Arc<VideoRelocator>,
    clock: Arc<dyn Clock>,
    videos_path: impl Into<String>,
) -> Self;

// domain/task/task.rs
pub enum Task {
    // ...existing variants
    SyncVideo { video_id: String }, // task_type "sync_video", payload {"video_id"}
}
impl Task {
    pub fn decode_sync_video_payload(payload: &str) -> Result<String, TaskError>;
    // lane_for("sync_video") == TaskLane::VideoSync
    // exclusivity_key("sync_video", ..) == Some("video:<id>")
    /// False for sync_video: a running sync must not swallow a newer request.
    pub fn dedupes_running(task_type: &str) -> bool;
}

// domain/task/task_lane.rs
pub enum TaskLane { Download, Thumbnail, Subtitles, VideoSync, Light, Exclusive }

// application/tasks/sync_video_task.rs
pub struct SyncVideoTask { video_reconciler: VideoReconciler }
impl TaskHandler for SyncVideoTask {
    fn handle(&self, payload: &str, is_last_attempt: bool) -> anyhow::Result<()>;
}

// application/subscribers/sync_video_on_video_event.rs
pub struct SyncVideoOnVideoEvent {
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}
impl EventSubscriber for SyncVideoOnVideoEvent {
    fn handle(&self, payload: &str) -> anyhow::Result<()>; // reads {"video_id"}
}
```

```ts
// web/src/lib/tasks.ts
// describeTask: 'sync_video' -> `Syncing ${videoInContainer(payload)}`
// FAMILY_BY_TYPE: sync_video: 'syncs'
```

YouTube lane decision: no new limit. Download (configurable N), thumbnail (1) and subtitle (1) lanes stay global; per-video mutual exclusion already makes a video's YouTube calls sequential.

## Call Stack

**Video added** (`VideoAddedToChannel { channel_id, video_id }` / `VideoAddedToPlaylist`)
1. `SyncVideoOnVideoEvent::handle(payload)` → `TaskRepository::schedule(Task::SyncVideo { video_id }, clock.now())`
2. executor (lane `VideoSync`, key `video:<id>`) → `SyncVideoTask::handle(payload, _)` → `Task::decode_sync_video_payload` → `VideoRecordId::new`
3. `VideoReconciler::reconcile(&video_id)`
   - `VideoRepository::find(&video_id)` → `None` ⇒ `Ok(())`
   - `ChannelVideoRepository::find_by_video` → `ChannelRepository::find(&channel_id)`, else `PlaylistVideoRepository::find_by_video` → `PlaylistRepository::find(&playlist_id)` ⇒ `VideoTarget { output_dir: videos_path/owner.path, quality: owner.quality }`; neither ⇒ `Ok(())`
   - `reconcile_video(&video, &target)` → `VideoSyncPlan::for_video(&video).steps` → for each `TaskRepository::schedule(&step.task_for(&video, &target), clock.now())`

**Video downloaded** (`VideoDownloaded { video_id, output_dir, folder }`)
1. `SyncVideoOnVideoEvent::handle` → schedule `SyncVideo` (Plex scan subscriber unchanged)
2. → same as above; plan is `[FetchSubtitles]` (thumbnail recorded by the download).

**Reconcile pass** (`ReconcileChannelTask` / `ReconcilePlaylistTask` → `InternalVideoReconciler::reconcile(desired, delta)`)
1. `read_actual_state(desired)`; `target = VideoTarget { output_dir: actual.output_dir, quality: desired.quality }`
2. `redownload_broken_files` / `recover_errored_videos` → `VideoRepository::update(reset)` → `VideoReconciler::schedule_download(&reset, &target)`
3. `generate_missing_metadata` (unchanged)
4. every `actual.videos` not in `delta.added_ids` ∪ reset ids → `VideoReconciler::reconcile_video(video, &target)`
5. `delete_orphaned_files` (unchanged)

**Schedule dedupe** — `SqliteTaskRepository::schedule(task, run_at)`: when `Task::exclusivity_key` is `Some`, skip if a task of the same type and `video_id` is `pending`, or also `running` when `Task::dedupes_running(type)`.

## Test Plan

Behaviour tests (application layer):

`application/tasks/sync_video_task.rs`
1. `it_should_schedule_the_download_and_thumbnail_of_a_new_channel_video` — Pending channel video ⇒ `download_video` (channel quality, output dir) + `fetch_thumbnail` pending.
2. `it_should_schedule_the_download_of_a_new_playlist_video_in_its_playlist_folder` — playlist owner resolves quality/output dir.
3. `it_should_schedule_the_subtitles_of_a_downloaded_video` — Downloaded, numbered, thumbnail set, Missing ⇒ only `fetch_subtitles`.
4. `it_should_schedule_nothing_for_a_fully_synced_video` — Downloaded + thumbnail + Fetched ⇒ no task.
5. `it_should_not_schedule_a_second_download_if_one_is_queued` — existing pending download ⇒ still one.
6. `it_should_not_schedule_a_thumbnail_for_a_video_being_downloaded` — InProgress ⇒ download (deduped) only.
7. `it_should_schedule_nothing_for_a_settled_errored_video` — Errored ⇒ no task.
8. `it_should_skip_if_video_is_gone` — unknown id ⇒ `Ok`, no task.
9. `it_should_skip_if_video_has_no_owner` — video not in any channel/playlist ⇒ `Ok`, no task.

`application/subscribers/sync_video_on_video_event.rs`
10. `it_should_schedule_a_video_sync_when_a_video_is_added_to_a_channel`
11. `it_should_schedule_a_video_sync_when_a_video_is_added_to_a_playlist`
12. `it_should_schedule_a_video_sync_when_a_video_is_downloaded`

`application/tasks/reconcile_channel_task.rs` (existing suite stays green; new)
13. `it_should_schedule_only_the_download_of_a_video_reset_for_redownload` — reset video ⇒ `download_video`, no `fetch_thumbnail`.
14. `it_should_schedule_no_step_for_a_video_added_by_the_same_pass` — only `sync_video` (via its event), no direct step tasks.

`application/http/tasks/mod.rs` (list handler)
15. `it_should_list_a_video_sync_with_its_video_title_and_owner_name`

Domain unit (`domain/task/task.rs`)
16. `it_should_run_video_syncs_in_their_own_lane`
17. `it_should_give_a_video_sync_the_video_exclusivity_key`

Web (`web/src/lib/tasks.test.ts`)
18. `describes a video sync with its video and container`
19. `puts video syncs in the syncs family`

Infrastructure tests:

`infrastructure/repositories/sqlite_task_repository.rs`
20. `it_should_not_add_a_video_sync_if_one_is_pending`
21. `it_should_add_a_video_sync_if_the_only_one_is_running`
