# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every type and signature from design.md's Types & Signatures and wire them end to end. Rust: `TaskView.started_at`; `TaskResponse.started_at`; `RunTasksNowRequest`; `TaskRepository::list_pending_by_ids` (SQLite stub returns an empty `Vec`, plus the in-memory fake); `TaskRunNowRequester` + `TaskRunNowRequesterApi`; the `run_tasks_now` handler (answers 204); the routes; `ApiServices` fields; construction in `serve.rs`. Web: `Task.started_at` and the `aTask` builder; `runTasksNow` in `client.ts`; `lib/taskTimes.ts` and `lib/taskErrors.ts` stubs; the new `lib/tasks.ts` exports, with trivial bodies next to the existing ones; `components/tasks/{TaskFilterChips,TaskRow,TaskBatchRow,AttentionCard,useTaskAction}` rendering nothing yet and not mounted. Done when `cargo build` and `npm run typecheck` succeed and all existing tests pass. No new behaviour, no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_include_video_id_for_a_download_task`: the listing adds `payload.video_id` for per-video tasks
- [x] 2.2 `it_should_include_video_id_when_the_video_no_longer_exists`: `video_id` comes from the stored payload, not from the resolved video
- [x] 2.3 `it_should_not_include_video_id_for_a_ytdlp_update`: tasks that don't work on a video have no `video_id`
- [x] 2.4 `it_should_include_started_at_for_a_running_task`: `started_at` is the claim time (`updated_at`) while the task runs
- [x] 2.5 `it_should_have_null_started_at_for_a_pending_task`
- [x] 2.6 `it_should_bring_a_pending_task_forward_when_run_now`: `POST /api/tasks/run-now` answers 204 and the task is due now
- [x] 2.7 `it_should_bring_every_listed_task_forward`: all listed pending tasks become due now
- [x] 2.8 `it_should_ignore_running_and_unknown_ids_when_run_now`: 204, and running and unknown ids are left alone
- [x] 2.9 `it_should_keep_retries_and_last_error_when_run_now`: only the run time changes
- [x] 2.10 `it_should_reject_run_now_without_ids`: an empty or missing `ids` answers 400
- [x] 2.17 Web `taskFamily`: downloads, syncs, AI on and off, Plex & upkeep, unknown type (`lib/tasks.test.ts`)
- [x] 2.18 Web `describeTask`: base form when pending, "-ing" when running, every task type, placeholders for missing names; no "pending" or "queued" wording
- [x] 2.19 Web `sectionOf`: running, attention (pending with `last_error`), next hour (including already due), later
- [x] 2.20 Web `batchReconciles`: a chain within 10 min is one batch, a gap over 10 min splits it, a lone reconcile stays a task, errored or running reconciles are never batched
- [x] 2.21 Web `buildTaskList`: chip counts add up to All, empty chips hidden, AI chip only while AI is enabled, filter and query (description and a batch's source names), empty sections dropped
- [x] 2.22 Web `statusLine`: idle with the next task due, next task already due ("starting soon"), "Downloading N video(s)", "Running N tasks", "N more tasks are scheduled"
- [x] 2.23 Web `formatRunTime` / `formatStartedAgo` (`lib/taskTimes.test.ts`): today, tomorrow, already due → "Starting soon", "started 12 s ago"
- [x] 2.24 Web `explainTaskError` (`lib/taskErrors.test.ts`): 403, private, age-gated, members-only, unavailable, unknown → first line without "ERROR:"
- [x] 2.25 Web `TasksView` opens on All with the status line and sections for a prod-like fixture, and shows no status badges (rewrite `TasksView.test.tsx`; delete the tab tests)
- [x] 2.26 Web: selecting a chip filters the list; a chip with no tasks is hidden; chips are marked pressed when selected
- [x] 2.27 Web: a batch row expands and collapses, and **Sync all now** posts every id of the batch to `/api/tasks/run-now`
- [x] 2.28 Web: **Sync now** on one source posts its id; **Run now** on a Plex sync posts its id; per-video and running tasks offer no Run now
- [x] 2.29 Web: the Needs attention card shows the plain reason and the next try; **Show error** / **Hide error** toggles the raw text
- [x] 2.30 Web: **Retry now** posts to `/api/videos/{id}/retry`, refreshing the list and disabling itself while in flight
- [x] 2.31 Web: a failed run-now or retry shows the failure and the action can be used again
- [x] 2.32 Web: search is always shown, filters within the selected chip, keeps its text when the chip changes, and says nothing matches when nothing does
- [x] 2.33 Web: an empty listing shows "All quiet. Nothing is scheduled." with no chips
- [x] 2.34 Web: the phone layout keeps chip names in a sideways-scrolling row and stacks each row's time above its description; the controls use the 44 px size classes (asserted on classes in jsdom, and checked for real in 4.3)
- [x] 2.37 Web: a failing non-video task's Needs attention card offers **Run now** (from audit)
- [x] 2.38 Web: the selected chip falls back to **All** once it has no tasks left (`buildTaskList` and `TasksView`) (from audit)
- [x] 2.39 Web: every task button has `aria-describedby` pointing at its task, batch or source description (from audit)

## 3. Infrastructure adapters (TDD)

- [x] 3.1 `SqliteTaskRepository` `it_should_list_only_pending_tasks_among_the_given_ids`: pending, running and missing ids → only the pending task
- [x] 3.2 `SqliteTaskRepository` `it_should_list_nothing_for_an_empty_id_list`

## 4. Verification

- [x] 4.1 Run `scripts/run-tests.sh --lint --web --rust` and verify that every suite passes
- [x] 4.2 Update the e2e specs for chips and sections, then run `scripts/run-tests.sh --e2e` and verify they pass. Files: `e2e-tests/tests/tasks.spec.js` ("Update yt-dlp" under Plex & upkeep, no "pending" badge), `e2e-tests/helpers/tasks.js` (`expectPendingSync` via the Syncs chip and the clock / "in N min" text), `mobileLayout.spec.js` (Syncs chip by visible name, then Run now), `erroredDownload.spec.js` (Needs attention card, Retry now), `search.spec.js` (tasks search with fewer than 15 tasks), and the "download listed while running" check (Running now)
- [x] 4.3 Manual (agent). Start `scripts/run-local.sh --port <free port> --fresh-videos`, add 3+ channels and playlists with short intervals through the HTTP API, and open `/tasks` with the Playwright MCP browser at 1440×900 and 390×844. Assert: the status line text; chips and counts; Next hour / Later sections; a batch expands and lists its sources; **Sync all now** sends one `POST /api/tasks/run-now` holding every id (network log) and the batch then shows under Running now / Starting soon; on a phone every chip's name is visible and controls are ≥44 px tall (bounding boxes). Take a screenshot of each width. Then stop the server and delete `yarrtube.sqlite3` and the fresh videos dir
- [ ] 4.4 Manual (human): compare the result side by side with the design canvas (https://claude.ai/artifact/36GX5JmSxQNgjTnyxpcirs) in light and dark mode for spacing, contrast and the amber Needs attention card, and check on a real phone that the chip row scrolls sideways smoothly
- [ ] 4.5 Manual (human): against prod-like data with a download YouTube refuses (403), check that the Needs attention card's reason reads well and that **Retry now** runs the download again

## 5. Drop Stop retrying

- [x] 5.1 Remove the backend: `VideoRetryStopper` (+ module), `StopRetryingVideoError`, `stop_retrying` on `StepState`, each step and `Video`, `Video::retrying_steps` / `step_state`, the `stop_retrying_video` handler, the `/videos/{id}/stop-retrying` route, the `ApiServices` field and its `serve.rs` wiring, and their tests (`it_should_*retrying_is_stopped`, `it_should_change_nothing_when_no_step_is_retrying`, `it_should_answer_404_when_stopping_retrying_an_unknown_video`, `it_should_make_a_stopped_download_pending_again_when_the_video_is_retried`)
- [x] 5.2 Remove the web side: `stopRetryingVideo` in `client.ts`, the **Stop retrying** button (the card keeps **Retry now**), and Stop retrying in the component tests (the chip-fallback test drains the Downloads chip another way)
- [x] 5.3 Remove the Stop retrying step from `e2e-tests/tests/erroredDownload.spec.js`
- [x] 5.4 Run `scripts/run-tests.sh --lint --web --rust` and `--e2e`, and verify they pass

## Workflow follow-up

- Run `/audit-change`, then `/openspec:finish` to sync specs, archive and open the PR.
