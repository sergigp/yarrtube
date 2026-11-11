# Design

## Files

- `src/domain/video/events.rs`: `VideoSummaryGenerated`, next to the video's other step events (summary state lives on `Video`).
- `src/domain/video/mod.rs`: re-export it.
- `src/domain/event/domain_event.rs`: `DomainEvent::VideoSummaryGenerated` variant, type and payload arms, mapping test.
- `src/domain/services/video_summarizer.rs`: takes an `EventPublisher`; publishes once the summary is stored and the video marked Done.
- `src/application/subscribers/sync_video_on_video_summary_generated.rs` (new): schedules `SyncVideo` now, copy of `SyncVideoOnVideoSubtitlesFetched`.
- `src/application/subscribers/mod.rs`: register it under `video_summary_generated`.
- `src/serve.rs`: pass `infrastructure.event_publisher` to `VideoSummarizer::new`.
- `src/application/tasks/summarize_video_task.rs`: test helper wires a `SqliteEventPublisher`; new tests.
- `src/domain/preferences/preferences.rs`: `changes_categorization` compares every call setting.
- `src/application/http/preferences/mod.rs`: new test.
- `src/domain/services/video_categorizer.rs`: `Skip::NothingPending`, checked in `prepare` before the server check.
- `src/application/tasks/categorize_video_task.rs`: new tests.
- `doc/ARCHITECTURE.md` (step 7), `doc/AI.md` ("How a video is categorized"): the sync after a summary, the wider reset, the no-call skip.

## Types & Signatures

```rust
// src/domain/video/events.rs
/// A video's summary was stored and its summary step is Done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoSummaryGenerated {
    pub video_id: String,
}
impl VideoSummaryGenerated {
    pub const EVENT_TYPE: &str = "video_summary_generated";
}

// src/domain/event/domain_event.rs
pub enum DomainEvent { /* … */ VideoSummaryGenerated(VideoSummaryGenerated), /* … */ }

// src/domain/services/video_summarizer.rs
pub struct VideoSummarizer { /* … */ event_publisher: Arc<dyn EventPublisher>, /* … */ }
impl VideoSummarizer {
    pub fn new(
        video_repository, video_owner_finder, preferences_repository,
        video_transcript_repository, video_file_repository, llm_server_repository,
        video_summary_repository,
        event_publisher: Arc<dyn EventPublisher>,   // new, before clock
        clock,
    ) -> Self;
    /// Best effort, like `VideoSubtitlesFetched`: a lost event only delays
    /// the categorization to the next reconcile pass.
    fn publish_generated(&self, video: &Video);
}

// src/application/subscribers/sync_video_on_video_summary_generated.rs
#[derive(Debug, Deserialize)]
struct VideoSummaryGeneratedPayload { video_id: String }
pub struct SyncVideoOnVideoSummaryGenerated {
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}
impl SyncVideoOnVideoSummaryGenerated {
    pub fn new(task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self;
}
impl EventSubscriber for SyncVideoOnVideoSummaryGenerated {
    fn handle(&self, payload: &str) -> anyhow::Result<()>;
}

// src/domain/preferences/preferences.rs
impl Preferences {
    /// Whether `other` categorizes differently: another server or other
    /// settings (model, context length as saved, temperature, thinking,
    /// effective prompt), not merely categorization turned on or off.
    pub fn changes_categorization(&self, other: &Self) -> bool; // signature unchanged
}

// src/domain/services/video_categorizer.rs
enum Skip {
    NotWanted(&'static str),
    /// No category is pending for the video: nothing to ask, so it is
    /// categorized as it stands (Done), without checking the server.
    NothingPending,
    Blocked(anyhow::Error),
    Unavailable(anyhow::Error),
    Failed(anyhow::Error, Box<CategorizationJob>),
}
```

## Call Stack

**Summary stored → sync**
1. `SummarizeVideoTask::handle` → `VideoSummarizer::summarize(&video)` → `generate` → `try_generate`
2. `still_wanted(video)` → `None` → return, no event
3. `VideoSummaryRepository::save(&VideoSummary)` → `VideoUpdater::update(&id, mark_summary_generated(now) if is_summarizable)`
4. update returned `Some(_)` → `publish_generated(video)` → `EventPublisher::publish(&DomainEvent::VideoSummaryGenerated { video_id })` (warn on `Err`); `None` → no event
5. `DomainEventsConsumer` → `SyncVideoOnVideoSummaryGenerated::handle(r#"{"video_id":..}"#)` → `TaskRepository::schedule(&Task::SyncVideo { video_id }, clock.now())`
6. `SyncVideo` → `VideoReconciler` → `categorization_wanted` → `schedule_steps` → `Task::CategorizeVideo` (unchanged)

**Preferences saved**
1. `PATCH /api/preferences` → `PreferencesUpdater::update(change)` → `stored.changes_categorization(&updated)` → `reset_failed_categorizations()` (unchanged)
2. `VideoCategorizer::unchanged_since(job, category)` → `!job.preferences.changes_categorization(&current)` (same method, now wider)

**Categorization with nothing pending**
1. `CategorizeVideoTask::handle` → `VideoCategorizer::categorize(&video)` → `prepare(&video)`
2. … owner, summary found → `self.pending(&video, &owner)` → empty → `Err(Skip::NothingPending)` (before `check_server`)
3. `categorize` → `info!("skipped video categorization", reason = "no category is pending")` → `mark_categorized(&video)` → `VideoUpdater::update(&id, mark_categorized(now))` → `Ok(VideoStepOutcome::Settled)`

## Test Plan

1. Behaviour tests
   1. `it_should_map_video_summary_generated_to_a_stable_type_and_payload` (`domain_event.rs`, existing per-event pattern): type `video_summary_generated`, payload `{ "video_id": "rec1" }`.
   2. `it_should_schedule_a_video_sync` (`sync_video_on_video_summary_generated.rs`): `list_non_completed()` == `[(Task::SyncVideo { video_id: "rec1" }, now)]`.
   3. `it_should_announce_the_stored_summary` (`summarize_video_task.rs`): one-call summary stored → `SqliteEventRepository::list_eligible()` == `[pending_event(1, VideoSummaryGenerated { video_id })]`.
   4. `it_should_not_announce_a_dropped_summary` (`summarize_video_task.rs`): summaries turned off during the call → `list_eligible()` == `[]`.
   5. `it_should_mark_the_categorization_done_without_asking_if_nothing_is_pending` (`categorize_video_task.rs`): Retrying video (2 failed attempts), its only prompted category already has a user verdict, `FakeLlmServerRepository::unreachable()` → `Ok(TaskOutcome::Done)`, `llm.chats()` == `[]`, stored video == `Video { version: 1, categorization: StepState { Done, RetryAttempts::default() }, updated_at: later(), ..video }`.
   6. `it_should_reset_failed_verdicts_when_the_categorization_generation_settings_change` (`http/preferences/mod.rs`): for each of `categorization_context_length`, `categorization_temperature`, `categorization_thinking` changed → `failed` verdict deleted, `yes`/`no` kept, video categorization Pending (same asserts as the model-change test).
   7. `it_should_not_mark_a_category_failed_when_the_generation_settings_changed_during_its_attempt` (`categorize_video_task.rs`): temperature saved from `with_on_chat` during the last attempt, "Tech" rejected → no `failed` verdict, "Tech" pending. Expected to pass once 6 is green (same method); kept as regression.

   Toggling categorization on is already covered by `it_should_keep_failed_verdicts_when_the_same_settings_are_saved`.

2. Infrastructure tests: none (no adapter changes).
