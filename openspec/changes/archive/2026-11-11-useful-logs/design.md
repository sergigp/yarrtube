# Design

Non-obvious decisions, one line each:

- Every new duration (task run, queue wait, event dispatch, reconcile listing and total) is measured with the injected `Clock`, not `Instant`, so `FixedClock` makes the logged values deterministic and assertable. The categorizer's and summarizer's existing `Instant` timings stay.
- Log lines are asserted with the existing `log_capture` helpers (the repo's pattern for a log line that is a behaviour's only side effect). Executor tests call the private `execute` directly on the test thread, since `spawn_blocking` threads are not captured.
- No existing line moves to debug for now (the log is read with debug filtered out); every new line is at info.
- `steps_due` on `synced video` lists the plan's due steps; `TaskRepository::schedule` dedupes silently, so a step already queued still shows. Exact "inserted" reporting would need a write that returns a value, against the repository rules.
- Heartbeat counts come from entity-returning reads (`list_non_completed`, `list_pending`, `list_dead_lettered`, `list`), not new `count_*` SQL: a library of ~100s of videos read every 5 minutes is cheap. Dead-letter and video `list` already exist as `#[cfg(test)]` inherent methods; they move onto the ports.

## Files

- `src/domain/task/task_outcome.rs`: new `TaskOutcome::Unchanged` (done, nothing changed; logged as `outcome="unchanged"`).
- `src/domain/task/task.rs`: `keyed_video_id` → public `video_id`; new `channel_id`, `playlist_id` for log fields.
- `src/infrastructure/repositories/task_executor.rs`: `task finished` end line; failure lines gain the same fields; timings from `Clock`.
- `src/infrastructure/repositories/sqlite_task_repository.rs`: `list_dead_lettered` onto `TaskRepository`.
- `src/infrastructure/repositories/domain_events_consumer.rs`: `event finished` end line; failure lines gain fields.
- `src/infrastructure/shared/domain_events/event_repository.rs`: `list_dead_lettered` onto `EventRepository`.
- `src/infrastructure/repositories/sqlite_video_repository.rs`: `list` onto `VideoRepository`.
- `src/domain/video/video_sync_changes.rs`: what a video sync did.
- `src/domain/services/video_reconciler.rs`: `reconcile` returns `VideoSyncChanges`, logs `synced video` when not empty.
- `src/application/tasks/sync_video_task.rs`: empty changes → `TaskOutcome::Unchanged`, else `Done`.
- `src/domain/shared/membership_changes.rs`: what a pass's YouTube membership sync did (shared by both reconcilers).
- `src/domain/services/internal_video_reconciler.rs`: `reconcile` returns `FolderReconciliation` (syncs requested, orphans deleted).
- `src/domain/services/channel_video_reconciler.rs`, `playlist_video_reconciler.rs`: `reconciled …` summary line.
- `src/domain/video_category/categorization_tally.rs`: per-run counts.
- `src/domain/services/video_categorizer.rs`: `categorized video` summary; `thinking` on `asked the model`.
- `src/domain/services/video_summarizer.rs`: `thinking`, `output_tokens` on `model answered`.
- `src/domain/daemon/daemon_health.rs` (+ `mod.rs`): heartbeat read model.
- `src/domain/services/daemon_health_searcher.rs`: builds `DaemonHealth` from the repositories.
- `src/serve.rs`: `HEARTBEAT_INTERVAL` 5 min; `heartbeat_loop` takes the searcher, logs `daemon heartbeat` (warn with error on failure).
- `src/application/tasks/log_capture.rs`: `captured_info_lines` (info and above only).
- `doc/AI.md`, `doc/INSTALLATION.md`: the new end, summary and heartbeat lines.

## Types & Signatures

```rust
// domain/task/task_outcome.rs
pub enum TaskOutcome {
    Done,
    /// Done, and the run changed nothing: the end line reports `unchanged`.
    Unchanged,
    RunAgainAt { run_at: DateTime<Utc>, reason: String },
}

// domain/task/task.rs
impl Task {
    pub fn video_id(&self) -> Option<&str>;
    pub fn channel_id(&self) -> Option<&str>;   // ReconcileChannel, DeleteChannelFiles
    pub fn playlist_id(&self) -> Option<&str>;  // ReconcilePlaylist, DeletePlaylistFiles
}

// task_executor.rs (private); queued = claimed_at - run_at, run = clock.now() - claimed_at
fn execute(&self, running: ScheduledTask, claimed_at: DateTime<Utc>) -> anyhow::Result<()>;
fn record_failure(&self, running: ScheduledTask, error: anyhow::Error, claimed_at: Option<DateTime<Utc>>) -> anyhow::Result<()>;
// fields on every end line: task_id, task_type, video_id?, channel_id?, playlist_id?,
// outcome, run_seconds?, queued_seconds?, (+ run_at, reason | retries, error)

// domain_events_consumer.rs: fields on every end line:
// event_id, event_type, payload, subscriber_count, outcome, duration_seconds (+ retries, run_at, error)

// TaskRepository / EventRepository / VideoRepository (ports)
fn list_dead_lettered(&self) -> anyhow::Result<Vec<DeadLetteredTask>>;
fn list_dead_lettered(&self) -> anyhow::Result<Vec<DeadLetteredEvent>>;
fn list(&self) -> anyhow::Result<Vec<Video>>;

// domain/video/video_sync_changes.rs
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VideoSyncChanges {
    pub steps_due: Vec<VideoSyncStep>,
    pub reset_for_redownload: bool,
    pub metadata_repaired: bool,
    pub relocated: bool,
}
impl VideoSyncChanges { pub fn is_empty(&self) -> bool; }

// video_reconciler.rs
pub trait VideoReconcilerApi: Send + Sync {
    fn reconcile(&self, video_id: &VideoId) -> anyhow::Result<VideoSyncChanges>;
}
fn reset_if_broken(&self, video: Video, target: &VideoTarget) -> anyhow::Result<(Video, bool)>;
fn repair_metadata(&self, video: &Video, owner: &VideoOwner) -> VideoSyncChanges; // metadata_repaired, relocated
fn schedule_steps(&self, ..) -> anyhow::Result<Vec<VideoSyncStep>>;

// domain/shared/membership_changes.rs
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MembershipChanges {
    pub listed: Option<usize>,   // None: listing failed, membership untouched
    pub added: usize,
    pub removed: usize,
    pub listing_seconds: f64,
}

// internal_video_reconciler.rs (above the struct, next to DesiredState)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderReconciliation { pub syncs_requested: usize, pub orphans_deleted: usize }
pub trait InternalVideoReconcilerApi: Send + Sync {
    fn reconcile(&self, desired: &DesiredState) -> anyhow::Result<FolderReconciliation>;
}

// channel_video_reconciler.rs / playlist_video_reconciler.rs (private)
fn sync_membership_with_youtube(&self, .., on_listing_failure: ListingFailure) -> anyhow::Result<MembershipChanges>;
fn log_pass(channel: &Channel, membership: &MembershipChanges, folder: &FolderReconciliation, duration_seconds: f64);

// domain/video_category/categorization_tally.rs
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CategorizationTally { pub asked: usize, pub yes: usize, pub no: usize, pub rejected: usize }
impl CategorizationTally {
    pub fn new(pending: &[Category], verdicts: &[VideoCategoryVerdict], rejected: usize) -> Self;
}

// domain/daemon/daemon_health.rs
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StepCounts { pub pending: usize, pub retrying: usize, pub failed: usize }
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonHealth {
    pub pending_tasks: BTreeMap<&'static str, usize>,   // per lane name, lanes with 0 omitted
    pub running_tasks: usize,
    pub oldest_eligible_task_age_seconds: Option<i64>,
    pub pending_events: usize,
    pub dead_lettered_tasks: usize,
    pub dead_lettered_events: usize,
    pub downloads: StepCounts,
    pub summaries: StepCounts,
    pub categorizations: StepCounts,
}
// TaskLane gains `pub fn as_str(&self) -> &'static str`.

// domain/services/daemon_health_searcher.rs
pub struct DaemonHealthSearcher {
    task_repository: Arc<dyn TaskRepository>,
    event_repository: Arc<dyn EventRepository>,
    video_repository: Arc<dyn VideoRepository>,
    clock: Arc<dyn Clock>,
}
pub trait DaemonHealthSearcherApi: Send + Sync {
    fn search(&self) -> anyhow::Result<DaemonHealth>;
}

// serve.rs
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(300);
async fn heartbeat_loop(searcher: Arc<DaemonHealthSearcher>);

// log_capture.rs (drops debug lines so assertions survive a later demotion)
pub(crate) fn captured_info_lines(body: impl FnOnce()) -> Vec<String>;
```

## Call Stack

Task attempt:
```
TaskExecutor::start(state, task)
  repository.claim(id, clock.now())            -> claimed_at = clock.now()
  spawn(running, slot) -> execute(running, claimed_at)
    info!("dispatching task")
    dispatch(&running) -> handler.handle(&task)
    Done | Unchanged -> repository.delete(id); log_finished(&running, "done" | "unchanged", claimed_at)
    RunAgainAt       -> repository.update(..);  log_finished(.., "requeued", run_at, reason)
    Err(e)           -> record_failure(running, e, Some(claimed_at))   // warn/error with fields
recover_running_tasks -> record_failure(task, e, None)                 // no timings
```

Event dispatch:
```
DomainEventsConsumer::poll_once()
  for event in repository.list_eligible():
    started = clock.now(); info!("dispatching event")
    subscribers.each(dispatch)
    ok   -> repository.delete(id); info!("event finished", outcome="done", duration)
    fail -> event.fail(..) -> update / dead_letter; warn!/error! with same fields
```

Video sync:
```
SyncVideoTask::handle(Task::SyncVideo{video_id})
  VideoReconciler::reconcile(&video_id) -> VideoSyncChanges
    reset_if_broken -> (video, reset)
    repair_metadata -> {metadata_repaired, relocated}
    schedule_steps  -> steps_due
    if !changes.is_empty(): info!("synced video", video_id, steps_due, reset_for_redownload, metadata_repaired, relocated)
  changes.is_empty() ? TaskOutcome::Unchanged : TaskOutcome::Done
```

Reconcile pass (playlist identical, with `kind`):
```
ChannelVideoReconciler::reconcile_channel(channel, on_listing_failure)
  started = clock.now(); info!("reconciling channel")
  sync_membership_with_youtube(..) -> MembershipChanges   (clock around list_current_videos)
  internal_video_reconciler.reconcile(&desired) -> FolderReconciliation
  log_pass(channel, &membership, &folder, clock.now() - started)   // info!("reconciled channel")
```

Categorization:
```
VideoCategorizer::ask_each(video, job)
  rejected = rejected_categories(video, job)
    ask -> log_call(.., thinking=job.thinking, stopped_at_limit)
    store -> info!("video categorized")
  tally = CategorizationTally::new(&job.pending, &verdict_repository.find_many([video.id]), rejected.len())
  info!("categorized video", video_id, asked, yes, no, rejected)
  mark_categorized | fail_attempt
```

Heartbeat:
```
serve: tokio::spawn(heartbeat_loop(Arc::new(daemon_health_searcher(infrastructure))))
  every 5 min: spawn_blocking(searcher.search())
    task_repository.list_non_completed()  -> pending per lane, running, oldest eligible (pending, run_at <= now)
    event_repository.list_pending()       -> pending_events
    task_repository.list_dead_lettered(), event_repository.list_dead_lettered()
    video_repository.list()               -> downloads / summaries / categorizations StepCounts
  Ok(h)  -> info!("daemon heartbeat", ..fields)
  Err(e) -> warn!("daemon heartbeat", error)
```

## Test Plan

1. Behaviour tests (adapters, plus one domain service with no testable adapter):
   - `sync_video_task`: `it_should_report_nothing_changed_if_the_video_needs_nothing` — fully downloaded video with every step done → `Ok(TaskOutcome::Unchanged)`, no `synced video` line.
   - `sync_video_task`: `it_should_log_the_steps_it_schedules` — pending video → `Ok(Done)`, one `INFO synced video … steps_due=[FetchThumbnail, Download] …` line.
   - `sync_video_task`: `it_should_log_a_reset_for_redownload` — downloaded video with missing file → `Ok(Done)`, `synced video` carries `reset_for_redownload=true`.
   - `sync_video_task`: `it_should_log_a_metadata_repair` — downloaded video without metadata → `synced video` carries `metadata_repaired=true`.
   - `sync_video_task`: `it_should_report_nothing_changed_if_the_video_is_gone` — unknown id → `Ok(Unchanged)`.
   - `reconcile_channel_task`: `it_should_log_a_pass_summary` — one listed new, one stored removed, one orphan file → one `INFO reconciled channel channel_id=… listed=… added=1 removed=1 syncs_requested=… orphans_deleted=1 listing_seconds=0.0 duration_seconds=0.0` after `reconciling channel` and the per-video lines.
   - `reconcile_channel_task`: `it_should_log_a_pass_summary_if_listing_fails` — failing listing → summary without `listed`, zero added/removed.
   - `reconcile_playlist_task`: `it_should_log_a_pass_summary` — same as channel, with `kind`.
   - `categorize_video_task`: `it_should_log_a_run_summary` — 3 pending, answers yes/no/no → `INFO categorized video … asked=3 yes=1 no=2 rejected=0` after the three `video categorized` lines.
   - `categorize_video_task`: `it_should_count_rejected_categories_in_the_run_summary` — one unusable answer → `rejected=1`.
   - `daemon_health_searcher` (behaviour test, only caller is the serve loop): `it_should_report_queue_and_library_counts` — seeded tasks/events/dead letters/videos → exact `DaemonHealth`.
   - `daemon_health_searcher`: `it_should_report_no_oldest_task_if_none_is_eligible` — only future pending tasks → `oldest_eligible_task_age_seconds: None`.
2. Infrastructure tests:
   - `task_executor`: `it_should_log_one_line_when_a_task_finishes` — task run_at 5 min before claim → `dispatching task` then one `task finished task_id=1 task_type="sync_video" video_id="…" outcome="done" run_seconds=0.0 queued_seconds=300.0`.
   - `task_executor`: `it_should_log_a_finished_task_that_changed_nothing_as_unchanged` — handler returns `Unchanged` → task deleted, `task finished … outcome="unchanged"`.
   - `task_executor`: `it_should_log_the_next_run_of_a_requeued_task` — `RunAgainAt` → `outcome="requeued"` with `run_at`, `reason`.
   - `task_executor`: `it_should_log_a_retry_with_the_task_fields` — failing handler → only the `WARN task failed, retrying` line, with type, ids, timings, retries, run_at, error.
   - `domain_events_consumer`: `it_should_log_one_line_when_an_event_finishes` — `dispatching event` then one `event finished` with id, type, payload, subscriber_count, outcome, duration.
   - `domain_events_consumer`: `it_should_log_a_failed_event_with_its_fields` — failing subscriber → only the warn line, with the same fields plus error.
