## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature in design.md (## Files, ## Types & Signatures), wired end to end:
  - migration 0028 registered, with the full value mapping, so existing rows load;
  - `StepStatus`, `RetryAttempts`, `StepState`, `VideoStepOutcome`, `TaskOutcome`, `RetryVideoError`;
  - `LateDataChecks` holding a `StepState`;
  - `Video`'s per-step fields replacing `SubtitleState`/`SummaryState`/settled flags/`last_errored_at`;
  - the `VideoStatus` and `Verdict` values renamed;
  - `TaskHandler::handle(&Task) -> anyhow::Result<TaskOutcome>` on all 16 handlers and their fakes, and the executor matching `Done`/`RunAgainAt`;
  - every step service returning `VideoStepOutcome`, with `is_last_attempt` removed everywhere;
  - `VideoRetrier` and `POST /api/videos/{id}/retry` routed and wired;
  - the repository and verdict-repository renames and additions, with every new step-state write going through `VideoUpdater`;
  - `has_work_in_progress` taking `now`;
  - the `steps` DTO, the TS types, `useRetryVideo`, `videoSteps.ts` stubs and `aVideo`.

  Bodies stay trivial: services keep today's behaviour mapped onto the new states (`Settled`/`RetryAt`); `retry` returns `Ok(())`. Done when `cargo build` and `npm run typecheck` succeed and every existing test passes, with tests adapted mechanically to the renamed values and fields.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_schedule_the_next_attempt_if_the_download_fails`
- [x] 2.2 `it_should_wait_longer_after_each_failed_attempt`
- [x] 2.3 `it_should_retry_daily_from_the_fourth_failure`
- [x] 2.4 `it_should_give_up_after_the_eleventh_failed_attempt`
- [x] 2.5 `it_should_record_the_failure_if_yt_dlp_cannot_run`
- [x] 2.6 `it_should_record_the_failure_if_the_download_cannot_be_placed`
- [x] 2.7 `it_should_mark_the_video_unavailable_if_the_failure_is_permanent`
- [x] 2.8 `it_should_requeue_without_downloading_if_the_next_attempt_is_not_due`
- [x] 2.9 `it_should_download_a_retrying_video_once_due`
- [x] 2.10 `it_should_skip_a_failed_video`
- [x] 2.11 `it_should_fail_if_the_video_cannot_be_read`
- [x] 2.12 `it_should_schedule_the_next_attempt_if_no_thumbnail_is_obtained`
- [x] 2.13 `it_should_fail_the_thumbnail_after_the_eleventh_failed_attempt`
- [x] 2.14 `it_should_mark_the_thumbnail_done_once_fetched`
- [x] 2.15 `it_should_schedule_the_next_check_if_the_subtitle_check_fails`
- [x] 2.16 `it_should_fail_the_subtitles_after_the_eleventh_failed_check`
- [x] 2.17 `it_should_keep_young_subtitles_pending_after_a_conclusive_check`
- [x] 2.18 `it_should_mark_old_subtitles_unavailable_without_captions`
- [x] 2.19 `it_should_schedule_the_next_check_if_the_chapters_fetch_fails`
- [x] 2.20 `it_should_fail_the_chapters_after_the_eleventh_failed_fetch`
- [ ] 2.21 `it_should_schedule_the_next_check_if_the_quality_check_fails`
- [ ] 2.22 `it_should_fail_the_quality_after_the_eleventh_failed_check`
- [ ] 2.23 `it_should_schedule_the_next_attempt_if_the_summary_is_rejected`
- [ ] 2.24 `it_should_count_ollama_down_as_a_failed_summary_attempt`
- [ ] 2.25 `it_should_fail_the_summary_after_the_eleventh_failed_attempt`
- [ ] 2.26 `it_should_mark_the_summary_done_once_generated`
- [ ] 2.27 `it_should_schedule_the_next_attempt_if_a_category_is_rejected`
- [ ] 2.28 `it_should_count_ollama_down_as_a_failed_categorization_attempt`
- [ ] 2.29 `it_should_fail_pending_categories_after_the_eleventh_failed_attempt`
- [ ] 2.30 `it_should_mark_the_categorization_done_when_nothing_is_rejected`
- [ ] 2.31 `it_should_schedule_the_download_of_a_retrying_video_once_due` (sync); remove `recover_if_due` and its tests
- [ ] 2.32 `it_should_not_schedule_steps_that_are_not_due` (sync)
- [ ] 2.33 `it_should_not_schedule_failed_steps` (sync)
- [ ] 2.34 `it_should_retry_a_failed_video` (http)
- [ ] 2.35 `it_should_retry_the_failed_steps_of_a_downloaded_video` (http)
- [ ] 2.36 `it_should_bring_forward_a_queued_retry` (http)
- [ ] 2.37 `it_should_fail_to_retry_an_unknown_video` (http)
- [ ] 2.38 `it_should_list_a_retrying_video_with_its_steps` (http)
- [ ] 2.39 `it_should_list_a_downloaded_video_with_a_failed_summary` (http)
- [ ] 2.40 `it_should_move_a_source_whose_only_unfinished_video_waits_for_a_retry` (http)
- [ ] 2.41 `it_should_reset_failed_summaries_when_the_summary_settings_change` (renamed, regression)
- [ ] 2.42 `it_should_reset_failed_verdicts_when_the_categorization_model_changes` (renamed, regression)
- [ ] 2.43 Web `videoSteps.test.ts`: one cycle per case (labels, retrying line, failed line, overdue, `canRetry`); `npm run test` green
- [ ] 2.44 Web `VideoDetail.test.tsx`: one cycle per case (failing-step lines, "Retry now" posts and refetches, no button when healthy, retry error); then the components reading `EXCLUDED`/`unavailable` updated; `npm run test` green

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 task_executor: `it_should_requeue_a_task_its_handler_asks_to_run_again`
- [ ] 3.2 task_executor: `it_should_dead_letter_a_task_whose_handler_fails_on_the_fifth_attempt` (regression, new signature)
- [ ] 3.3 sqlite_migrations: `it_should_map_download_statuses_when_migrating`
- [ ] 3.4 sqlite_migrations: `it_should_backfill_the_download_failure_from_the_download_task_when_migrating`
- [ ] 3.5 sqlite_migrations: `it_should_map_subtitle_states_when_migrating`
- [ ] 3.6 sqlite_migrations: `it_should_map_settled_chapters_and_quality_when_migrating`
- [ ] 3.7 sqlite_migrations: `it_should_map_summary_states_and_verdicts_when_migrating`
- [ ] 3.8 sqlite_video_repository: `it_should_round_trip_every_step_state`
- [ ] 3.9 sqlite_video_category_verdict_repository: `it_should_delete_only_the_failed_verdicts_of_a_video`

## 4. Verification

- [ ] 4.1 e2e: update `helpers/video.js` (labels `RETRYING` "Retrying", `FAILED` "Failed", `UNAVAILABLE` "Unavailable"; fail fast on either of the first two while waiting for a download) and `tests/erroredDownload.spec.js`:
  - after the first failure the video shows "Retrying", "next attempt in 10 minutes" and the reason;
  - the Downloads tab lists its download pending with the reason;
  - "Retry now" runs it again; after the next failure it shows "next attempt in 1 hour". This covers `POST /api/videos/{id}/retry`.

  Update `doc/INSTALLATION.md` (`YARRTUBE_RETRY_BASE_DELAY_SECONDS`, back up the database before upgrading).
- [ ] 4.2 `scripts/run-tests.sh --only web --only rust` passes.
- [ ] 4.3 `scripts/run-tests.sh --only e2e` passes.
- [ ] 4.4 Manual (agent):
  1. Start `scripts/run-local.sh --port <free port> --fresh-videos`.
  2. Seed a playlist whose video fails, open it with the Playwright MCP browser, and assert "Retrying", "next attempt in 10 minutes" and the reason, plus the `steps.download` entry in `/api/playlists/{id}/videos`.
  3. Set the download to `FAILED` with 11 attempts in the local SQLite database, reload, and assert "Failed" and "gave up after 11 attempts".
  4. Click "Retry now" and assert the network `POST` and the status becoming Pending/Downloading.
  5. Stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
