## Files

- `src/domain/video/video.rs`: `unrecorded_folder_candidates` replaces `in_flight_download_folders`. The "which folders may still be written" rule belongs to the entity.
- `src/domain/services/playlist_video_reconciler.rs`: the membership sync returns `MembershipChanges` (added ids plus renamed videos' previous titles) and writes a title only when it changed; the orphan sweep protects every video's unrecorded folder candidates, including those from the previous title.
- `src/domain/services/channel_video_reconciler.rs`: the same, for channels.
- `src/infrastructure/repositories/task_executor.rs`: `SchedulerState` tracks running task ids; every scheduling pass recovers `running` rows it isn't running; startup recovery shares that code and `record_failure`.
- `src/infrastructure/shared/sqlite_connection.rs`: `TestDatabase::path`, so a test can build the production infrastructure.
- `src/serve.rs`: `task_handlers` and `event_subscribers` split out of the executor and consumer builders, so the production registries can be tested.
- Tests: `src/application/tasks/reconcile_playlist_task.rs`, `src/application/tasks/reconcile_channel_task.rs`, `src/infrastructure/repositories/task_executor.rs`, `src/serve.rs`.

## Types & Signatures

```rust
// src/domain/video/video.rs
impl Video {
    /// Folders a download or thumbnail fetch may still be writing into: the
    /// `video_folder_candidates` of the title (and of `previous_title`, when
    /// given) while `filename` is None; empty once the download recorded it.
    pub fn unrecorded_folder_candidates(&self, previous_title: Option<&str>) -> Vec<String>;
}
```

```rust
// src/domain/services/{playlist,channel}_video_reconciler.rs (private)
struct MembershipChanges {
    added_ids: Vec<VideoRecordId>,
    previous_titles: HashMap<VideoRecordId, String>,
}
fn sync_playlist_membership(&self, playlist: &Playlist) -> anyhow::Result<MembershipChanges>;
fn reconcile_filesystem(&self, playlist: &Playlist, changes: &MembershipChanges) -> anyhow::Result<()>;
// channel: sync_channel_membership(&Channel), reconcile_filesystem(&Channel, &MembershipChanges)
```

```rust
// src/infrastructure/repositories/task_executor.rs
struct SchedulerState {
    running: HashMap<TaskLane, usize>,
    held_keys: HashSet<String>,
    running_ids: HashSet<i64>,
}
struct Slot { task_id: i64, lane: TaskLane, key: Option<String> }
impl TaskExecutor {
    fn recover_orphaned_tasks(&self, state: &SchedulerState) -> anyhow::Result<()>;
    fn recover_running_tasks(&self, in_progress: &HashSet<i64>, error: &str) -> anyhow::Result<()>;
}
```

```rust
// src/serve.rs
fn task_handlers(infrastructure: &InfrastructureContainer) -> HandlerRegistry;
fn event_subscribers(infrastructure: &InfrastructureContainer) -> SubscriberRegistry;
```

## Call Stack

**Reconcile pass (playlist; channel mirrors it):**
1. `run_reconcile_pass(playlist)` → `sync_playlist_membership(playlist)` → `MembershipChanges`.
   - New video: `save` + `VideoAddedToPlaylist`; id into `added_ids`.
   - Existing video: `video_repository.find(id)`; if `title != current.title` → `update_title(id, current.title, now)`, and `previous_titles[id] = old title`.
2. `reconcile_filesystem(playlist, &changes)`: protected set = recorded filenames ∪ recorded thumbnail folders ∪ `video.unrecorded_folder_candidates(changes.previous_titles.get(&video.id))` for every stored video; `schedule_missing(videos, skip = added_ids ∪ reset ids)`; delete what is left unprotected.

**Scheduling pass:**
1. `schedule_pass` → `lock_state()` → `recover_orphaned_tasks(&state)` → `recover_running_tasks(&state.running_ids, "recovered as a failed attempt: left running with no attempt in progress")` → for each `list_running()` row not in `running_ids`: `record_failure(task, error)` (retry or dead-letter).
2. Then the lane / key / exclusive logic as before; `SchedulerState::start` and `finish` add and remove the task id.

**Startup:** `recover_stuck_tasks()` → `recover_running_tasks(&HashSet::new(), "recovered as a failed attempt after an unclean shutdown")`.

## Test Plan

**Behaviour tests**

*`ReconcilePlaylistTask`:*
1. `it_should_keep_the_folder_of_a_thumbnail_fetch_in_progress`: a Pending video with no thumbnail and its `My Video/` folder on disk. Nothing is deleted.
2. `it_should_keep_the_folder_of_a_download_in_progress_if_video_renamed`: an In Progress video renamed on YouTube, its old-title folder on disk. Nothing is deleted, and the video row has the new title.

*`ReconcileChannelTask`:*
3. `it_should_keep_the_folder_of_a_thumbnail_fetch_in_progress`: channel equivalent of 1.
4. `it_should_keep_the_folder_of_a_download_in_progress_if_video_renamed`: channel equivalent of 2.

*`TaskExecutor`:*
5. `it_should_retry_a_task_left_running_without_waiting_for_a_restart`: a `running` row the executor isn't running. After one pass it is pending with `retries: 1`, backoff `run_at` and the recovery error, and the handler was not called.
6. `it_should_start_the_next_task_as_soon_as_one_finishes`: `run` with a 1h interval and two light tasks. Both finish, so a completion triggers the next pass.

*Composition (`serve.rs`):*
7. `it_should_register_a_handler_for_every_task_type`: the production handler registry's keys are exactly the 8 task types.
8. `it_should_register_the_subscribers_of_every_event_type`: the production subscriber registry maps each of the 8 event types to its subscriber count (2 for each video-added event).

**Infrastructure tests**

None: no adapter changes.
