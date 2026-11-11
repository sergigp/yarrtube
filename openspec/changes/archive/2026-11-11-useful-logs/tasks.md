# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures): `TaskOutcome::Unchanged` (executor treats it like `Done`), `Task::{video_id, channel_id, playlist_id}`, `TaskLane::as_str`, `VideoSyncChanges`, `MembershipChanges`, `FolderReconciliation`, `CategorizationTally`, `DaemonHealth`/`StepCounts`, `DaemonHealthSearcher` (returns `DaemonHealth::default()`), `list_dead_lettered` on `TaskRepository`/`EventRepository` and `list` on `VideoRepository` (moved from `#[cfg(test)]`), `VideoReconcilerApi::reconcile` → `VideoSyncChanges::default()`, `InternalVideoReconcilerApi::reconcile` → `FolderReconciliation::default()`, `execute`/`record_failure` taking `claimed_at`, `captured_info_lines`, `heartbeat_loop` taking the searcher (wired in `serve.rs`). No log level or line changes yet. Done when `cargo build` succeeds and `cargo test --locked` passes.

## 2. Behaviour (TDD)

- [x] 2.1 `sync_video_task::it_should_report_nothing_changed_if_the_video_needs_nothing` — a no-op sync returns `Unchanged` and logs no `synced video` line (updates existing no-op tests' expected outcome).
- [x] 2.2 `sync_video_task::it_should_log_the_steps_it_schedules` — `synced video` with `steps_due`, outcome `Done`.
- [x] 2.3 `sync_video_task::it_should_log_a_reset_for_redownload` — `reset_for_redownload=true`.
- [x] 2.4 `sync_video_task::it_should_log_a_metadata_repair` — `metadata_repaired=true` (and `relocated` when a legacy video moves).
- [x] 2.5 `sync_video_task::it_should_report_nothing_changed_if_the_video_is_gone` — `Unchanged`.
- [x] 2.6 `reconcile_channel_task::it_should_log_a_pass_summary` — `reconciled channel` with counts and durations, after `reconciling channel` and the per-video lines.
- [x] 2.7 `reconcile_channel_task::it_should_log_a_pass_summary_if_listing_fails` — no `listed`, zero added/removed.
- [x] 2.8 `reconcile_playlist_task::it_should_log_a_pass_summary` — `reconciled playlist` with `kind`.
- [x] 2.9 `categorize_video_task::it_should_log_a_run_summary` — `categorized video` counts after the per-category `video categorized` lines.
- [x] 2.10 `categorize_video_task::it_should_count_rejected_categories_in_the_run_summary` — `rejected=1`.
- [x] 2.11 `daemon_health_searcher::it_should_report_queue_and_library_counts` — exact `DaemonHealth` (covers the three promoted port reads).
- [x] 2.12 `daemon_health_searcher::it_should_report_no_oldest_task_if_none_is_eligible` — `None` age.

## 3. Infrastructure adapters (TDD)

TaskExecutor:
- [x] 3.1 `it_should_log_one_line_when_a_task_finishes` — `dispatching task` then one `task finished` line with ids, outcome, `run_seconds`, `queued_seconds`.
- [x] 3.2 `it_should_log_a_finished_task_that_changed_nothing_as_unchanged` — `Unchanged` deletes the task, `task finished … outcome="unchanged"`.
- [x] 3.3 `it_should_log_the_next_run_of_a_requeued_task` — `outcome="requeued"`, `run_at`, `reason`.
- [x] 3.4 `it_should_log_a_retry_with_the_task_fields` — only the warn line, with type, ids, timings (dead-letter error line gets the same fields in the refactor step).

DomainEventsConsumer:
- [x] 3.5 `it_should_log_one_line_when_an_event_finishes` — `dispatching event` then one `event finished` line.
- [x] 3.6 `it_should_log_a_failed_event_with_its_fields` — only the warn line, with the same fields plus error.

## 4. Remaining edits and verification

- [x] 4.1 Add `thinking` to the categorizer's `asked the model` and `thinking` + `output_tokens` to the summarizer's `model answered` (no test: both lines carry an `Instant` duration; checked by reading the diff). Verify `cargo test --locked` passes.
- [x] 4.2 Heartbeat: 5-minute interval, `daemon heartbeat` info line with every `DaemonHealth` field, warn with the error when `search` fails. Verify by `cargo build` and reading the diff (serve loop has no test).
- [x] 4.3 Docs: `doc/AI.md` (summarizer/categorizer call fields, categorization run summary) and `doc/INSTALLATION.md` Logs section (one end line per task/event with outcome and timings, `outcome="unchanged"` for no-op syncs, reconcile summaries, 5-minute heartbeat with counts). Verify by reading the rendered sections.
- [x] 4.4 Run `scripts/run-tests.sh --only rust` (fmt, clippy, tests); all pass.
- [x] 4.5 Manual (agent): `scripts/run-local.sh --port <free port> --fresh-videos`, track a small playlist via the HTTP API, wait for its first pass and downloads, and check the log: one `task finished` per task with `queued_seconds`/`run_seconds`, one `event finished` per event, one `reconciled playlist`, a `daemon heartbeat` with counts once 5 minutes have passed, and a no-op `sync_video` (trigger reconcile again) ending with `outcome="unchanged"` and no `synced video` line. Then stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
