## Context

See proposal.md, "Why", for the incident. The current state that shapes the design:

- **Two background loops.** `serve.rs` spawns two independent `spawn_blocking` loops: `DomainEventsConsumer::run` and `TaskExecutor::run`. Each loop is strictly sequential on its own, but the two loops already run concurrently with each other. For example, a `PlaylistCreated` reconcile running in the consumer can overlap with a `DownloadVideo` running in the executor.
- **Executor dispatch.** `TaskExecutor::poll_once` snapshots `list_eligible()` (ordered by `run_at, id`), then for each task does `update(running)` → `handler.handle` → `delete`, `update(retry)` or `dead_letter`. The executor is task-agnostic: it maps `task_type` strings to `TaskHandler`s.
- **Inline thumbnail fetch.** Both reconcilers call `ThumbnailFetcher::fetch` inline for each new video (`playlist_video_reconciler.rs:222`, `channel_video_reconciler.rs:211`). They also call `fetch_missing` inline in `reconcile_filesystem`. Each fetch spawns `yt-dlp --skip-download --write-thumbnail` and takes about 17s.
- **Download scheduling.** `DownloadVideo` tasks are scheduled by `DownloadVideoOnVideoAddedTo{Playlist,Channel}` subscribers. Recovery paths inside the reconcilers schedule them directly.
- **Per-video folders.** The per-video folder is resolved in infrastructure (`ytdlp::prepare_video_dir` → `resolve_folder_collision`): the sanitized title, or `"{title} [{youtube_id}]"` if that name already exists on disk. It is only persisted on the video when the download completes (`filename`) or a thumbnail is recorded (`thumbnail_filename`).
- **Whole-row writes.** `VideoRepository::update` writes the whole row. Both the reconcile title refresh (`find` → `update(Video { title, ..video })`) and `ThumbnailFetcher::record_thumbnail` write back a snapshot read earlier.
- **Storage.** Each repository has its own SQLite connection, with WAL and a 5s `busy_timeout`. `rusqlite` is `bundled`, so `RETURNING` and `json_extract` are available.

## Goals / Non-Goals

**Goals:**
- A reconcile pass never runs `yt-dlp` per video, so video-added events and their downloads get scheduled in seconds.
- Downloads run in parallel, with a small configurable cap. Neither thumbnails nor reconciles/deletes wait behind downloads, and reconciles/deletes don't wait behind a thumbnail backlog.
- Every race that parallelism makes routine is closed first. Several of them exist today.
- The executor stays generic: it knows lanes and keys, not specific handlers.

**Non-Goals:**
- Moving the `PlaylistCreated`/`ChannelCreated` reconcile out of the event handler. It stays inline, as decided: once thumbnails are removed from it, it only does API and DB work.
- Parallelizing the event consumer.
- Fetching thumbnails without `yt-dlp` (e.g. direct `i.ytimg.com` URLs). This could be a follow-up, and it would only make `FetchThumbnail` tasks faster.
- Task priorities beyond lanes.
- Persisting a video's folder before its download completes (see D5).
- Graceful shutdown that waits for running tasks. Crash recovery already covers several tasks left `running`.

## Decisions

### D1. `FetchThumbnail` is a task scheduled by event subscribers

- **New variant:** `Task::FetchThumbnail { video_id, output_dir }` with task type `fetch_thumbnail`. `output_dir` is the resolved container directory, exactly as in `DownloadVideo`.
- **New subscribers:**
  - `FetchThumbnailOnVideoAddedToPlaylist` and `FetchThumbnailOnVideoAddedToChannel` mirror the download subscribers: they resolve the container, skip if it's gone, and schedule at `now`.
  - Each is registered before the matching download subscriber for the same event, so for a given video the thumbnail task gets the lower id.
- **New handler:** `FetchThumbnailTask`.
  - Re-reads the video by id and does nothing if the video is gone.
  - Otherwise calls `ThumbnailFetcher::fetch`, which already does nothing when `thumbnail_filename` is set.
  - Always returns `Ok`: the fetch swallows its own errors. This keeps today's best-effort semantics and avoids up to 5 queue retries of a 17s `yt-dlp` call for a video that simply has no thumbnail.
  - Retrying is left to the reconcile's missing-thumbnail recovery, as today.
- **Reconcilers:**
  - Both drop the inline `fetch` for new videos.
  - `ThumbnailFetcher::fetch_missing` becomes a scheduling pass. It keeps its current filters (no thumbnail, not in `skip_ids`, not `InProgress`) and schedules `FetchThumbnail` instead of fetching. Dedupe comes from D3, so the reconciler needs no `list_non_completed` lookup.
  - Recovery only covers videos stored before the pass. The membership sync returns the ids of the videos it added, and the reconciler adds them to `skip_ids` (next to the videos reset for redownload), so a new video's fetch comes only from its subscriber. Without this, the recovery pass schedules every new video's fetch first and the subscriber is always deduped away.
  - `ThumbnailFetcher` therefore needs the `TaskRepository` (for scheduling). Its synchronous `fetch` stays for the handler.
- **Alternative rejected:** scheduling `FetchThumbnail` directly inside the reconciler's new-video branch. The subscriber approach keeps the reconcile focused on recording facts, and matches how downloads are already triggered, which was your preference.

### D2. Lanes, per-video keys and an exclusive type are domain facts; the executor enforces them

In `domain::task`:

- `TaskLane { Download, Thumbnail, Light, Exclusive }`.
- `Task::lane_for(task_type: &str) -> TaskLane`:
  - `download_video` → `Download`,
  - `fetch_thumbnail` → `Thumbnail`,
  - `update_ytdlp` → `Exclusive`,
  - everything else, including unknown types → `Light`.
- `Task::exclusivity_key(task_type, payload) -> Option<String>`: returns `video:<video_id>` for `download_video` and `fetch_thumbnail`, and `None` otherwise.

`TaskExecutor` keeps its handler registry and gains an in-memory scheduler state behind a `Mutex`:

- running count per lane,
- the set of held exclusivity keys,
- an `exclusive_running` flag.

One scheduling pass (every `BACKGROUND_POLL_INTERVAL`, and also immediately whenever a task finishes, using a `tokio::sync::Notify`):

1. If `exclusive_running`, start nothing.
2. Read eligible tasks in `run_at, id` order.
3. If any eligible task is `Exclusive`:
   - if nothing is running, claim and start the first such task and stop;
   - otherwise start nothing (drain) and stop.
4. Otherwise, go through the eligible tasks in order. Skip a task if:
   - its lane is at capacity, or
   - its exclusivity key is held.

   Claim each remaining task atomically (D3). Once claimed, record its lane slot and key, then `spawn_blocking` its handler.
5. When a handler finishes (inside the blocking closure):
   - apply the existing success / retry / dead-letter bookkeeping,
   - release the slot and the key,
   - `notify` the scheduler.

Lane capacities: `Download` = `YARRTUBE_DOWNLOAD_CONCURRENCY` (default 2), `Thumbnail` = 1 (fixed), `Light` = 1, `Exclusive` = 1 (and alone).

The per-video key applies across lanes: a `fetch_thumbnail` (Thumbnail lane) and a `download_video` (Download lane) for the same video never run together.

- **Why an in-memory key set is enough:** there is exactly one executor per process, and the database is only shared with the HTTP layer (which never claims tasks). After a crash the key set is empty, and `recover_stuck_tasks` has already turned every `running` row back into a retry before the executor starts.
- **Why domain mapping:** the lane and the key follow from what a task does. Keeping them next to `Task::task_type()` puts all per-type facts in one place, while `TaskExecutor` stays free of any handler-specific code.
- **Alternatives rejected:**
  - A generic N-worker pool with no lanes. Thumbnails and reconciles would queue behind long downloads again, and every race would have to be handled across all task types.
  - Keeping thumbnails in the Light lane. A new 80-video playlist queues about 80 × 17s of thumbnail fetches, which would hold reconciles and deletes for about 20 minutes (e.g. a deleted playlist's files would stay on disk that long).
  - One tokio task per lane, each polling independently. It's simpler per lane, but then exclusivity ("drain everything, then run the update") needs cross-lane coordination anyway. A single scheduler with shared state is easier to reason about and to test.

### D3. Atomic claim and schedule-time dedupe in `SqliteTaskRepository`

- **`claim(id, now) -> Option<ScheduledTask>`:** `UPDATE tasks SET status='running', updated_at=? WHERE id=? AND status='pending' RETURNING …`.
  - It replaces the executor's `update(&running)`.
  - A task can't be dispatched twice, even if a scheduling pass overlaps with another (timer tick vs. completion notify).
- **`schedule` dedupes per-video types in one statement:** `INSERT INTO tasks (...) SELECT ... WHERE NOT EXISTS (SELECT 1 FROM tasks WHERE task_type = ?1 AND json_extract(payload, '$.video_id') = ?2 AND status IN ('pending','running'))`.
  - This applies only when `Task::exclusivity_key` is `Some`.
  - Every call site gets the dedupe for free: subscribers, reconcile recovery, the reconcile's reset-for-redownload.
  - An event retry that re-runs the download subscriber becomes harmless.
  - `schedule` keeps returning `Ok(())` whether or not a row was added.
- **Alternative rejected:** skipping at claim time when the video is `InProgress`. The status can be stale after a crash, which would strand the video. Dedupe plus the per-video key covers the same cases without that risk.

### D4. Targeted video updates

- `VideoRepository` gains `update_title(id, title, now)` and `update_thumbnail(id, thumbnail_filename, now)`. Each updates only that column plus `updated_at`.
- Where they're used:
  - the reconcilers' "existing video" branch uses `update_title` (position already lives in the separate `playlist_video`/`channel_video` rows),
  - `ThumbnailFetcher::record_thumbnail` uses `update_thumbnail`.
- The downloader's whole-row writes stay. The per-video key (D2) means no thumbnail fetch runs during a download of the same video. And the reconcile only whole-row-writes `Downloaded`/`Errored` videos when resetting them, never `InProgress` ones.

### D5. Orphan sweep protects in-flight download folders by predicted name

- In `reconcile_filesystem` (both reconcilers), also protect the two folder names a download of an `InProgress` or `ErroredRetrying` video can be using:
  - `VideoFilename::from_title(title)`,
  - `"{that} [{youtube_id}]"`.
- To make this possible, the collision-suffix format moves from `ytdlp::resolve_folder_collision` into the domain (`domain::video`), so the reconciler and the infrastructure share one definition.
- **Alternative considered:** persist the chosen folder on the video when its download starts. That's exact, but it needs a new column, a migration, and moving folder resolution (which touches the filesystem) ahead of the `yt-dlp` call. Name prediction reaches the same safety with no schema change.
- **Cost:** a genuinely stale folder that happens to share a name with an in-flight video survives one more pass. It will be swept once that video is `Downloaded` and records its real folder.

### D6. Atomic folder creation for concurrent same-title downloads

In `ytdlp::prepare_video_dir`, when there is no `existing_folder`:

1. Create the parent with `create_dir_all`.
2. Try `std::fs::create_dir(desired)`.
3. On `AlreadyExists`, use the suffixed name, which is unique per video, and create it with `create_dir_all`.

This makes check-and-create a single filesystem operation, so two concurrent downloads with the same title can't both pick the bare name. It also keeps today's naming exactly (`video-naming` is unchanged). `fetch_thumbnail` shares `prepare_video_dir`, so it gets the same fix.

### D7. Download finishing for a deleted video cleans up after itself

In `VideoDownloader::download`, after `run_download` succeeds, re-read the video:

- **Gone** (removed from its container, or its whole playlist or channel deleted, which removes its record): delete `output_dir/<downloaded.folder>` through `VideoFileRepository` (best-effort, logged) and return `Ok(())` without recording anything.
- **Present:** record the download as today.

The failure paths are already safe:

- `yt-dlp` removes a folder it created on failure (`remove_video_dir_unless_reused`).
- The retry finds the video gone and skips.
- `mark_errored` on a missing row updates nothing.

Concurrent `DeletePlaylistFiles` needs no change beyond this.

### D8. Configuration

`serve.rs` gains `download_concurrency()`, which reads `YARRTUBE_DOWNLOAD_CONCURRENCY` like the other env readers. It keeps only values greater than 0 and defaults to 2. The value is passed into `TaskExecutor::new`. The README env table documents it, with a note on YouTube throttling.

### D9. Task listing and UI

- `TaskViewSearcher` resolves `fetch_thumbnail` the same way as `download_video`: video title, plus the name of the playlist or channel the video belongs to.
- `TasksView.jsx` gets a `fetch_thumbnail` case: "Fetching thumbnail of {video} in {container}".

## Risks / Trade-offs

- **[YouTube throttling or bot checks]** At most N parallel downloads plus one thumbnail fetch, so N+1 concurrent `yt-dlp` sessions (a `ReconcileChannel` in the Light lane can add one more for channel discovery). → Default N = 2, configurable, documented. The Thumbnail lane is fixed at 1. The exclusive update never overlaps with any of them.
- **[SQLite write contention]** More concurrent writers. → All writes are short statements on separate WAL connections with a 5s busy timeout, and no connection lock is held across a handler.
- **[Title refresh during a download defeats D5's prediction]** If the video's title changes on YouTube while its download is running, the predicted names no longer match the folder in use. → Rare. The window is one download. Acceptable, given the schema-free approach.
- **[Tasks tab gets twice as long]** Every new video now queues two tasks. → They are small, and the thumbnail tasks drain quickly in their own lane.
- **[Thumbnail can arrive after the download]** If the download lane claims a video first, its thumbnail task waits (per-video key) and then does nothing. → Correct by design: the download writes its own thumbnail.
- **[Rollback]** An older binary has no `fetch_thumbnail` handler, so leftover tasks fail with "no handler registered" and are dead-lettered after 5 attempts. → Harmless. They can also be purged manually.

## Migration Plan

- No schema migration.
- Queued tasks from before the upgrade (`download_video`, etc.) keep working.
- On first start, the recurring reconciles will schedule `FetchThumbnail` for any video still missing a thumbnail.
- Rollback: redeploy the previous image (see the rollback risk above).

## Files

**Domain**
- `src/domain/task/task.rs`:
  - new `Task::FetchThumbnail { video_id, output_dir }` (task type `fetch_thumbnail`), with payload encoding and `decode_fetch_thumbnail_payload`;
  - new `Task::lane_for` and `Task::exclusivity_key`.
- `src/domain/task/task_lane.rs` (new): the `TaskLane` enum. Re-exported from `domain::task`.
- `src/domain/video/video_filename.rs` (or the module that owns folder naming): new `video_folder_candidates(title, youtube_id)`, returning the bare folder name and the `"{name} [{youtube_id}]"` suffixed one. `ytdlp.rs` reuses the suffix format from here.
- `src/domain/services/thumbnail_fetcher.rs`:
  - `ThumbnailFetcher::new` takes a `TaskRepository`;
  - `fetch_missing` becomes `schedule_missing` and schedules `FetchThumbnail` instead of fetching;
  - `record_thumbnail` uses `update_thumbnail`.
- `src/domain/services/playlist_video_reconciler.rs` and `channel_video_reconciler.rs`:
  - drop the inline `fetch` for new videos;
  - use `update_title` for existing videos;
  - protect `video_folder_candidates` of `InProgress`/`ErroredRetrying` videos in the orphan sweep;
  - call `schedule_missing`.
- `src/domain/services/video_downloader.rs`: after a successful `run_download`, re-read the video; if it's gone, delete the folder the download wrote.
- `src/domain/services/task_view_searcher.rs`: resolve `fetch_thumbnail` like `download_video`.

**Application**
- `src/application/subscribers/fetch_thumbnail_on_video_added_to_playlist.rs` (new).
- `src/application/subscribers/fetch_thumbnail_on_video_added_to_channel.rs` (new).
- `src/application/subscribers/mod.rs`: register both, each before the download subscriber for the same event.
- `src/application/tasks/fetch_thumbnail_task.rs` (new).
- `src/application/tasks/mod.rs`: register `fetch_thumbnail`. The registry takes a `ThumbnailFetcher` and a `VideoRepository`.

**Infrastructure**
- `src/infrastructure/repositories/sqlite_task_repository.rs`:
  - `claim`;
  - `schedule` dedupes per-video task types with `INSERT … SELECT … WHERE NOT EXISTS`.
- `src/infrastructure/repositories/sqlite_video_repository.rs`: `update_title`, `update_thumbnail`. The fakes (if any) get the same methods.
- `src/infrastructure/repositories/task_executor.rs`:
  - lanes, the scheduler state, `Notify`, the exclusive drain;
  - `run` drives scheduling passes;
  - `poll_once` is replaced by `schedule_pass`.
- `src/infrastructure/repositories/youtube_video_downloader_repository.rs`: `FakeVideoDownloaderRepository` gains an optional `on_download` hook, so a test can delete the video mid-download.
- `src/infrastructure/shared/ytdlp.rs`: `prepare_video_dir` creates the bare folder atomically with `create_dir` and falls back to the suffixed name on `AlreadyExists`.

**Composition, docs, frontend**
- `src/serve.rs`:
  - `download_concurrency()`;
  - `thumbnail_fetcher()` passes the task repository;
  - `task_executor()` passes the concurrency and the new registry deps.
- `README.md`: document `YARRTUBE_DOWNLOAD_CONCURRENCY`.
- `web/src/components/TasksView.jsx`: `fetch_thumbnail` case.

## Types & Signatures

```rust
// src/domain/task/task_lane.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskLane { Download, Thumbnail, Light, Exclusive }

// src/domain/task/task.rs
pub enum Task {
    // ...existing variants
    FetchThumbnail { video_id: String, output_dir: String },
}
impl Task {
    pub fn lane_for(task_type: &str) -> TaskLane;
    /// `Some("video:<id>")` for `download_video` and `fetch_thumbnail`.
    pub fn exclusivity_key(task_type: &str, payload: &str) -> Option<String>;
    pub fn decode_fetch_thumbnail_payload(payload: &str) -> Result<(String, String), TaskError>;
}
```

```rust
// src/domain/video/…
/// The folder names a download of this video may be writing into: the bare
/// sanitized title, and the collision-suffixed one.
pub fn video_folder_candidates(title: &str, youtube_id: &VideoId) -> [String; 2];
pub fn collision_suffixed_folder(folder: &str, youtube_id: &str) -> String;
```

```rust
// src/domain/services/thumbnail_fetcher.rs
impl ThumbnailFetcher {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self;
}
pub trait ThumbnailFetcherApi: Send + Sync {
    fn fetch(&self, video: &Video, output_dir: &Path);
    /// Schedules a `FetchThumbnail` for each video with no thumbnail, not in
    /// `skip_ids` and not `InProgress`. Dedupe is left to `TaskRepository::schedule`.
    fn schedule_missing(
        &self,
        videos: &[Video],
        skip_ids: &HashSet<&VideoRecordId>,
        output_dir: &Path,
    ) -> anyhow::Result<()>;
}
```

```rust
// src/application/subscribers/fetch_thumbnail_on_video_added_to_playlist.rs
pub struct FetchThumbnailOnVideoAddedToPlaylist { /* playlist_repository, task_repository, clock, videos_path */ }
impl FetchThumbnailOnVideoAddedToPlaylist {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self;
}
impl EventSubscriber for FetchThumbnailOnVideoAddedToPlaylist { fn handle(&self, payload: &str) -> anyhow::Result<()>; }
// fetch_thumbnail_on_video_added_to_channel.rs: same shape, with ChannelRepository.

// src/application/tasks/fetch_thumbnail_task.rs
pub struct FetchThumbnailTask { /* video_repository, thumbnail_fetcher */ }
impl FetchThumbnailTask {
    pub fn new(video_repository: Arc<dyn VideoRepository>, thumbnail_fetcher: Arc<ThumbnailFetcher>) -> Self;
}
impl TaskHandler for FetchThumbnailTask { fn handle(&self, payload: &str, is_last_attempt: bool) -> anyhow::Result<()>; }
```

```rust
// src/infrastructure/repositories/sqlite_task_repository.rs
pub trait TaskRepository: Send + Sync {
    /// Adds the task, unless it has an exclusivity key and a task of the same
    /// type with the same key is already pending or running (then a no-op).
    fn schedule(&self, task: &Task, run_at: DateTime<Utc>) -> anyhow::Result<()>;
    /// Atomically flips a pending task to running; `None` if it was no longer pending.
    fn claim(&self, id: i64, now: DateTime<Utc>) -> anyhow::Result<Option<ScheduledTask>>;
    // ...existing methods
}

// src/infrastructure/repositories/sqlite_video_repository.rs
pub trait VideoRepository: Send + Sync {
    fn update_title(&self, id: &VideoRecordId, title: &str, now: DateTime<Utc>) -> anyhow::Result<()>;
    fn update_thumbnail(&self, id: &VideoRecordId, thumbnail_filename: &str, now: DateTime<Utc>) -> anyhow::Result<()>;
    // ...existing methods
}
```

```rust
// src/infrastructure/repositories/task_executor.rs
pub struct TaskExecutor {
    repository: Arc<dyn TaskRepository>,
    handlers: HandlerRegistry,
    clock: Arc<dyn Clock>,
    base_retry_delay_seconds: i64,
    download_concurrency: usize,
    state: Mutex<SchedulerState>,
    wake: Notify,
}
struct SchedulerState {
    running: HashMap<TaskLane, usize>,
    held_keys: HashSet<String>,
    exclusive_running: bool,
}
impl TaskExecutor {
    pub fn new(
        repository: Arc<dyn TaskRepository>,
        handlers: HandlerRegistry,
        clock: Arc<dyn Clock>,
        base_retry_delay_seconds: i64,
        download_concurrency: usize,
    ) -> Self;
    /// One scheduling pass: claims and spawns every eligible task that fits
    /// (lane capacity, held keys, exclusive drain). Returns the handles of the
    /// tasks it started, so tests can await them.
    pub fn schedule_pass(self: &Arc<Self>) -> anyhow::Result<Vec<tokio::task::JoinHandle<()>>>;
    pub fn recover_stuck_tasks(&self) -> anyhow::Result<()>;
    /// Runs a pass on every `interval` tick and on every task completion.
    pub async fn run(self: Arc<Self>, interval: Duration);
}
```

```rust
// src/serve.rs
const DEFAULT_DOWNLOAD_CONCURRENCY: usize = 2;
fn download_concurrency() -> usize; // YARRTUBE_DOWNLOAD_CONCURRENCY, > 0, else the default
```

## Call Stack

**New video in a playlist:**
1. `ReconcileOnPlaylistCreated` / `ReconcilePlaylistTask` → `PlaylistVideoReconciler::reconcile`, which saves the `Video` and `PlaylistVideo` and publishes `VideoAddedToPlaylist` (no `yt-dlp`).
2. `DomainEventsConsumer` dispatches that event to:
   - `FetchThumbnailOnVideoAddedToPlaylist` → `schedule(FetchThumbnail)`,
   - `DownloadVideoOnVideoAddedToPlaylist` → `schedule(DownloadVideo)`.
3. `TaskExecutor::schedule_pass`:
   - `FetchThumbnail` (Thumbnail lane, key `video:X`) → `FetchThumbnailTask` → `ThumbnailFetcher::fetch` → `update_thumbnail`;
   - `DownloadVideo` (Download lane, key `video:X`) waits while the key is held, then `DownloadVideoTask` → `VideoDownloader::download` runs, reusing the thumbnail's folder.

**Orphan sweep:** `reconcile_filesystem` builds the protected set from:
- the downloaded filenames,
- every recorded thumbnail folder,
- `video_folder_candidates` for `InProgress`/`ErroredRetrying` videos.

It then calls `schedule_missing` (skipping videos reset for redownload and videos the same pass added) and deletes whatever is left unprotected.

## Test Plan

Tests follow the repo's style: explicit arrange per test, a real `TestDatabase` and SQLite repositories, fakes only for external dependencies, and whole typed asserts, including side effects (the task table, video rows, fake calls).

**Behaviour tests**

*Subscribers:*
1. `fetch_thumbnail_on_video_added_to_playlist.rs` › `it_should_schedule_a_thumbnail_fetch`: the playlist exists. `list_non_completed()` returns one `fetch_thumbnail` task with `{ video_id, output_dir: "/videos/<path>" }`, due now.
2. `fetch_thumbnail_on_video_added_to_playlist.rs` › `it_should_skip_if_playlist_no_longer_exists`: returns `Ok(())` and schedules no task.
3. `fetch_thumbnail_on_video_added_to_channel.rs` › `it_should_schedule_a_thumbnail_fetch`: same as 1, for a channel.
4. `fetch_thumbnail_on_video_added_to_channel.rs` › `it_should_skip_if_channel_no_longer_exists`: same as 2, for a channel.

*`FetchThumbnailTask` (`fetch_thumbnail_task.rs`):*
5. `it_should_fetch_and_record_the_thumbnail`: the fake returns `FetchedThumbnail`. The video row equals the seeded one plus `thumbnail_filename` and `updated_at`.
6. `it_should_skip_if_video_already_has_a_thumbnail`: the fake's thumbnail calls are empty and the row is unchanged.
7. `it_should_skip_if_video_no_longer_exists`: returns `Ok(())` and there are no fake calls.
8. `it_should_succeed_without_recording_if_thumbnail_fetch_fails`: the fake errors. Returns `Ok(())` and the row is unchanged.

*`ReconcilePlaylistTask` (`reconcile_playlist_task.rs`):*
9. `it_should_not_fetch_thumbnails_when_adding_new_videos`: a playlist with 2 new items. The videos are stored `Pending` with no thumbnail, 2 `video_added_to_playlist` events are published, and the fake downloader recorded no thumbnail calls. Existing tests that expected an inline thumbnail are updated to this.
10. `it_should_schedule_a_thumbnail_fetch_for_a_video_missing_one`: a stored `Pending` video with no thumbnail. Afterwards, one `fetch_thumbnail` task for it.
11. `it_should_not_schedule_a_second_thumbnail_fetch_if_one_is_queued`: the same video already has a pending `fetch_thumbnail`. Still exactly one.
12. `it_should_not_schedule_a_thumbnail_fetch_for_a_video_being_downloaded`: an `InProgress` video with no thumbnail. No `fetch_thumbnail` task.
13. `it_should_keep_the_folder_of_a_download_in_progress`: an `InProgress` video titled "Song" with no thumbnail, and a `Song/` folder on disk. The fake file repository's deleted list is empty.
14. `it_should_keep_the_suffixed_folder_of_a_download_in_progress`: same as 13, with `Song [ytid]/`.
15. `it_should_still_delete_a_folder_no_video_accounts_for`: an `Other/` folder with no matching video is deleted. This guards the sweep.

*`ReconcileChannelTask` (`reconcile_channel_task.rs`):*
16. `it_should_not_fetch_thumbnails_when_adding_new_videos`: same as 9, for channels.
17. `it_should_schedule_a_thumbnail_fetch_for_a_video_missing_one`: same as 10, for channels.
18. `it_should_keep_the_folder_of_a_download_in_progress`: same as 13, for channels.

*`DownloadVideoTask` (`download_video_task.rs`):*
19. `it_should_remove_the_folder_if_video_deleted_during_download`: the fake's `on_download` hook deletes the video row. Returns `Ok(())`, the fake file repository deleted `/videos/<path>/<folder>`, and no video row exists.

*Task listing (`src/application/http/tasks/mod.rs`):*
20. `it_should_list_a_fetch_thumbnail_task_with_video_and_playlist_names`: the response task has `payload { video_title, playlist_name }`.

*`TaskExecutor` (`task_executor.rs`):* fake handlers block on a `std::sync::mpsc` channel until released, and record starts in a shared `Vec`. The tests are `#[tokio::test(flavor = "multi_thread")]`.
21. `it_should_run_downloads_in_parallel_up_to_the_lane_concurrency`: concurrency 2, three downloads for different videos. After one pass, the first two have started and the third hasn't. Release one, run a pass, and the third starts.
22. `it_should_not_block_light_tasks_behind_downloads`: concurrency 1, one blocked download running, a reconcile eligible. The reconcile starts.
23. `it_should_run_light_tasks_one_at_a_time`: two reconciles. Only the first starts until it is released.
24. `it_should_not_block_reconciles_behind_thumbnail_fetches`: a blocked `fetch_thumbnail` running, another queued, and a reconcile eligible. The reconcile starts.
25. `it_should_run_thumbnail_fetches_one_at_a_time`: two `fetch_thumbnail` tasks for different videos. Only the first starts until it is released.
26. `it_should_not_run_two_tasks_for_the_same_video_at_once`: a blocked `download_video` for X and an eligible `fetch_thumbnail` for X. The fetch doesn't start until the download is released.
27. `it_should_start_a_later_task_when_the_earlier_one_is_held_back`: a blocked `download_video` for X running, then an eligible `fetch_thumbnail` for X (held) followed by a `fetch_thumbnail` for Y. Y starts in the Thumbnail lane.
28. `it_should_run_update_ytdlp_only_when_nothing_else_runs`: a blocked download running and an eligible update. The update doesn't start, a newly eligible download doesn't start either, and once the running download is released the update runs alone.
29. `it_should_not_start_tasks_while_update_ytdlp_runs`: a blocked update running and an eligible reconcile. The reconcile doesn't start until the update is released.
30. `it_should_start_tasks_of_a_lane_in_run_at_order`: the existing ordering expectation, adapted to a lane.
31. The existing dispatch, retry, dead-letter, last-attempt and recovery tests are adapted to `schedule_pass` plus awaiting the returned handles.

*Domain (`task.rs`):*
32. `it_should_map_task_types_to_lanes`: `download_video` → `Download`, `fetch_thumbnail` → `Thumbnail`, `update_ytdlp` → `Exclusive`, `reconcile_playlist` and the delete tasks → `Light`.
33. `it_should_key_video_tasks_by_video_id`: `download_video` and `fetch_thumbnail` → `Some("video:<id>")`, `reconcile_playlist` → `None`.

*Composition (`serve.rs`):*
34. `it_should_default_download_concurrency_when_invalid`: `"0"`, `"-1"`, `"abc"` and unset all give 2, and `"4"` gives 4. The parser is a pure function taking `Option<String>`.

**Infrastructure tests**

*`SqliteTaskRepository`:*
35. `it_should_claim_a_pending_task`: returns `Some` with status `Running`, and the row is `running`.
36. `it_should_not_claim_a_task_already_running`: returns `None`.
37. `it_should_not_schedule_a_duplicate_download_for_the_same_video`: two schedules give one row.
38. `it_should_not_schedule_a_duplicate_thumbnail_fetch_for_the_same_video`: two schedules give one row.
39. `it_should_schedule_a_download_for_another_video`: two rows.
40. `it_should_schedule_duplicates_of_tasks_without_a_video`: two `reconcile_playlist` schedules give two rows.

*`SqliteVideoRepository`:*
41. `it_should_update_only_the_title`: an `InProgress` video. After `update_title`, `find` equals the original with the new `title` and `updated_at`.
42. `it_should_update_only_the_thumbnail`: same as 41, for `thumbnail_filename`.

*`ytdlp.rs` (temp dir):*
43. `it_should_give_concurrent_same_title_videos_distinct_folders`: two threads call `prepare_video_dir` for "Song" with different ids. The folders are `{"Song", "Song [idB]"}` in some order, and both exist.
44. The existing `it_should_append_the_video_id_to_the_thumbnail_folder_on_collision` and `it_should_reuse_the_same_folder_name_on_a_retry_after_a_failed_attempt` stay green unchanged.

**Frontend:** no unit test runner. Check `fetch_thumbnail` rows on the tasks view manually with `scripts/run-local.sh`, and keep `npm run build` and `npm run lint` in `web/` green. Smoke tests must stay green.
