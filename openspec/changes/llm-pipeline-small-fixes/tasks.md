# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md: `VideoSummaryGenerated` (+ re-export, `DomainEvent` variant with type/payload arms), `VideoSummarizer` taking an `EventPublisher` (wired in `src/serve.rs` and the `summarize_video_task` test helper with a `SqliteEventPublisher`), `publish_generated` stub doing nothing, `SyncVideoOnVideoSummaryGenerated` with `handle` returning `Ok(())` registered under `video_summary_generated` in `subscribers/mod.rs`, `Skip::NothingPending` (never returned yet; `categorize` arm settling). Done when `cargo build` succeeds and `cargo test --locked` passes unchanged.

## 2. Behaviour (TDD)

- [ ] 2.1 `it_should_map_video_summary_generated_to_a_stable_type_and_payload` (`domain_event.rs`): the event serializes as `video_summary_generated` with `{ "video_id" }`.
- [ ] 2.2 `it_should_schedule_a_video_sync` (`sync_video_on_video_summary_generated.rs`): the subscriber schedules `SyncVideo` for the video now.
- [ ] 2.3 `it_should_announce_the_stored_summary` (`summarize_video_task.rs`): a stored summary publishes `VideoSummaryGenerated`. Update `doc/ARCHITECTURE.md` step 7 and `doc/AI.md` (the video sync bullet) to say a stored summary syncs the video right away.
- [ ] 2.4 `it_should_not_announce_a_dropped_summary` (`summarize_video_task.rs`): a summary dropped because summaries were turned off during the call publishes nothing.
- [ ] 2.5 `it_should_mark_the_categorization_done_without_asking_if_nothing_is_pending` (`categorize_video_task.rs`): empty `pending` → no server check, no call, categorization Done with no failed attempts, task Done. Update `doc/AI.md` (the task-run bullet) with the skip.
- [ ] 2.6 `it_should_reset_failed_verdicts_when_the_categorization_generation_settings_change` (`http/preferences/mod.rs`): context length, temperature or thinking changed → `failed` verdicts deleted, categorization Pending. Update the `doc/AI.md` bullet "Saving a new Ollama URL, categorization model or prompt…" to list every call setting.
- [ ] 2.7 `it_should_not_mark_a_category_failed_when_the_generation_settings_changed_during_its_attempt` (`categorize_video_task.rs`): temperature changed during the last attempt → no `failed` verdict, category pending (expected to pass after 2.6; keep as regression).

## 3. Infrastructure adapters (TDD)

None: no adapter changes.

## 4. Verification

- [ ] 4.1 `scripts/run-tests.sh --only web --only rust` passes (fmt, clippy, tests).
