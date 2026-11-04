# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures) across all layers, wired end-to-end: migration `0023` registered, `summarize_video` handler and `SyncVideoOnVideoSubtitlesFetched` subscriber registered (including the `task_handlers` key test in `src/serve.rs`), `TaskLane::Summaries` in `Task::lane_for`, `VideoSummarizer` and `VideoSummaryRepository` constructed in `src/serve.rs` and `InfrastructureContainer`, plus the fakes (`FakeLlmServerRepository` recording chats with queued replies and an on-chat hook). Bodies are trivial (`Ok(())`, `None`, no-op `summarize`), never `todo!()`. Verify: `cargo build` succeeds and all existing tests pass; no new behaviour, no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `src/application/tasks/summarize_video_task.rs`: `it_should_store_the_summary_of_a_video_that_fits_one_call` — fake reply stored with model; state Generated; one chat. Verify: test red, then green.
- [x] 2.2 `src/application/tasks/summarize_video_task.rs`: `it_should_send_the_shipped_prompt_filled_with_the_videos_data` — exact system and user messages, `lang` from the language name. Verify: test red, then green.
- [x] 2.3 `src/application/tasks/summarize_video_task.rs`: `it_should_send_the_saved_prompt_when_there_is_one` — messages come from the saved text. Verify: test red, then green.
- [x] 2.4 `src/application/tasks/summarize_video_task.rs`: `it_should_send_the_saved_model_settings` — model, context, temperature, output cap in the chat. Verify: test red, then green.
- [x] 2.5 `src/application/tasks/summarize_video_task.rs`: `it_should_list_the_chapters_in_the_prompt` — `chapters` section, `0:00 Intro`. Verify: test red, then green.
- [x] 2.6 `src/application/tasks/summarize_video_task.rs`: `it_should_leave_chapters_out_of_the_prompt_when_the_video_has_none` — no block. Verify: test red, then green.
- [x] 2.7 `src/application/tasks/summarize_video_task.rs`: `it_should_keep_braces_in_values_unfilled` — title `{transcript} explained` unchanged. Verify: test red, then green.
- [x] 2.8 `src/application/tasks/summarize_video_task.rs`: `it_should_use_the_language_code_when_its_name_is_unknown` — `xh`. Verify: test red, then green.
- [x] 2.9 `src/application/tasks/summarize_video_task.rs`: `it_should_drop_the_reasoning_before_the_answer` — `</think>` stripped from the stored text. Verify: test red, then green.
- [x] 2.10 `src/application/tasks/summarize_video_task.rs`: `it_should_ask_for_thinking_only_when_the_model_supports_it` — table of setting × capability. Verify: test red, then green.
- [x] 2.11 `src/application/tasks/summarize_video_task.rs`: `it_should_summarize_a_long_transcript_in_parts_then_reduce` — N map chats + reduce holding answers in order. Verify: test red, then green.
- [x] 2.12 `src/application/tasks/summarize_video_task.rs`: `it_should_cut_parts_at_chapter_starts` — each part starts at a chapter; only overlapping chapters listed; `{span}`. Verify: test red, then green.
- [x] 2.13 `src/application/tasks/summarize_video_task.rs`: `it_should_cut_by_size_without_chapters` — parts under the budget, no text lost. Verify: test red, then green.
- [x] 2.14 `src/application/tasks/summarize_video_task.rs`: `it_should_cut_a_chapter_too_long_on_its_own_by_size` — others whole. Verify: test red, then green.
- [x] 2.15 `src/application/tasks/summarize_video_task.rs`: `it_should_scale_the_budget_with_the_context_length` — same text single at 12288, parts at 6144. Verify: test red, then green.
- [x] 2.16 `src/application/tasks/summarize_video_task.rs`: `it_should_do_nothing_when_summaries_are_no_longer_available` — table AI/summaries off; no chat. Verify: test red, then green.
- [x] 2.17 `src/application/tasks/summarize_video_task.rs`: `it_should_do_nothing_when_the_source_opted_out` — no chat, Missing. Verify: test red, then green.
- [x] 2.18 `src/application/tasks/summarize_video_task.rs`: `it_should_do_nothing_for_a_video_already_summarized` — no chat, summary unchanged. Verify: test red, then green.
- [x] 2.19 `src/application/tasks/summarize_video_task.rs`: `it_should_do_nothing_for_a_missing_video` — `Ok`. Verify: test red, then green.
- [x] 2.20 `src/application/tasks/summarize_video_task.rs`: `it_should_skip_without_counting_when_the_saved_prompt_is_invalid` — no chat, 0 attempts, no check. Verify: test red, then green.
- [x] 2.21 `src/application/tasks/summarize_video_task.rs`: `it_should_skip_without_counting_when_ollama_is_unreachable` — task `Ok`, no check. Verify: test red, then green.
- [x] 2.22 `src/application/tasks/summarize_video_task.rs`: `it_should_skip_without_counting_when_the_model_is_not_installed` — same. Verify: test red, then green.
- [x] 2.23 `src/application/tasks/summarize_video_task.rs`: `it_should_count_a_failed_attempt_when_the_chat_fails` — 1 attempt, check recorded, nothing stored. Verify: test red, then green.
- [x] 2.24 `src/application/tasks/summarize_video_task.rs`: `it_should_count_a_failed_attempt_when_the_answer_stopped_at_the_limit` — nothing stored. Verify: test red, then green.
- [x] 2.25 `src/application/tasks/summarize_video_task.rs`: `it_should_count_a_failed_attempt_when_the_input_was_truncated` — prompt tokens 12250. Verify: test red, then green.
- [x] 2.26 `src/application/tasks/summarize_video_task.rs`: `it_should_count_a_failed_attempt_when_the_answer_is_only_reasoning` — nothing stored. Verify: test red, then green.
- [x] 2.27 `src/application/tasks/summarize_video_task.rs`: `it_should_stop_at_the_first_failed_part` — no later chats, 1 attempt. Verify: test red, then green.
- [x] 2.28 `src/application/tasks/summarize_video_task.rs`: `it_should_count_a_failed_attempt_when_the_subtitle_file_is_gone` — no chat. Verify: test red, then green.
- [x] 2.29 `src/application/tasks/summarize_video_task.rs`: `it_should_become_unavailable_on_the_third_failure` — state Unavailable. Verify: test red, then green.
- [x] 2.30 `src/application/tasks/summarize_video_task.rs`: `it_should_store_nothing_when_the_video_is_deleted_while_generating` — fake chat hook deletes it. Verify: test red, then green.
- [x] 2.31 `src/application/tasks/sync_video_task.rs`: `it_should_schedule_a_summary_for_a_subtitled_video_in_an_opted_in_source` — one `summarize_video`. Verify: test red, then green.
- [x] 2.32 `src/application/tasks/sync_video_task.rs`: `it_should_not_schedule_a_summary_when_the_source_opted_out` — none. Verify: test red, then green.
- [x] 2.33 `src/application/tasks/sync_video_task.rs`: `it_should_not_schedule_a_summary_while_ai_summaries_are_off` — table. Verify: test red, then green.
- [x] 2.34 `src/application/tasks/sync_video_task.rs`: `it_should_not_schedule_a_summary_without_fetched_subtitles` — Missing, Unavailable. Verify: test red, then green.
- [x] 2.35 `src/application/tasks/sync_video_task.rs`: `it_should_space_summary_retries_by_a_day` — 3 h no, 25 h yes. Verify: test red, then green.
- [x] 2.36 `src/application/tasks/sync_video_task.rs`: `it_should_not_schedule_a_settled_summary` — Generated, Unavailable. Verify: test red, then green.
- [x] 2.37 `src/application/tasks/sync_video_task.rs`: `it_should_not_queue_a_second_summary` — dedupe. Verify: test red, then green.
- [x] 2.38 `src/application/tasks/reconcile_channel_task.rs`: `it_should_schedule_summaries_for_the_stored_videos_of_an_opted_in_channel` — one per video. Verify: test red, then green.
- [x] 2.39 `src/application/subscribers/sync_video_on_video_subtitles_fetched.rs`: `it_should_schedule_a_video_sync` — `sync_video` now. Verify: test red, then green.
- [x] 2.40 `src/application/http/tasks/mod.rs`: `it_should_list_a_summarize_video_task_with_video_and_channel_names`. Verify: test red, then green.
- [x] 2.41 `src/application/http/tasks/mod.rs`: `it_should_list_a_summarize_video_task_whose_video_is_gone` — no title. Verify: test red, then green.
- [x] 2.42 `src/application/http/playlists/mod.rs, channels/mod.rs`: `it_should_delete_the_summaries_of_a_deleted_playlist`. Verify: test red, then green.
- [x] 2.43 `src/application/http/playlists/mod.rs, channels/mod.rs`: `it_should_delete_the_summaries_of_a_deleted_channel`. Verify: test red, then green.
- [x] 2.44 `src/application/tasks/reconcile_channel_task.rs, reconcile_playlist_task.rs`: `it_should_delete_the_summary_of_a_video_removed_from_the_channel`. Verify: test red, then green.
- [x] 2.45 `src/application/tasks/reconcile_channel_task.rs, reconcile_playlist_task.rs`: `it_should_delete_the_summary_of_a_video_removed_from_the_playlist`. Verify: test red, then green.
- [x] 2.46 `src/domain/task/task.rs`: `it_should_run_summarize_video_in_the_summaries_lane_exclusive_per_video` — lane, key. Verify: test red, then green.
- [x] 2.47 `src/infrastructure/repositories/task_executor.rs`: `it_should_run_one_summary_at_a_time` — lane capacity 1. Verify: test red, then green.
- [x] 2.48 `src/infrastructure/repositories/task_executor.rs`: `it_should_not_hold_other_lanes_back_while_summarizing`. Verify: test red, then green.
- [x] 2.49 `web/src/lib/tasks.test.ts`: `describes a summarize task` — `Summarizing <video>`; fallback without a title. Verify: test red, then green.
- [x] 2.50 `src/application/tasks/summarize_video_task.rs (logging)`: `it_should_log_each_calls_prompt_answer_and_duration` — captured info logs hold both messages, the answer and `duration_seconds`. Verify: test red, then green.
- [x] 2.51 `src/application/tasks/summarize_video_task.rs (logging)`: `it_should_log_the_summary_total_time_and_video_length` — final entry fields. Verify: test red, then green.
- [x] 2.52 `src/application/tasks/summarize_video_task.rs (logging)`: `it_should_log_why_a_generation_was_skipped_or_failed` — info reason on skip, warn reason on failure. Verify: test red, then green.

## 3. Infrastructure adapters (TDD)

- [x] 3.1 `ollama_llm_server_repository`: `it_should_post_the_chat_with_its_options` — mockito body: model, messages, `stream: false`, `think`, `num_ctx`, `temperature`, `num_predict`. Verify: test red, then green.
- [x] 3.2 `ollama_llm_server_repository`: `it_should_read_the_answer_and_input_tokens`. Verify: test red, then green.
- [x] 3.3 `ollama_llm_server_repository`: `it_should_report_a_reply_stopped_at_the_limit` — `done_reason: "length"`. Verify: test red, then green.
- [x] 3.4 `ollama_llm_server_repository`: `it_should_fail_on_an_error_status`. Verify: test red, then green.
- [x] 3.5 `ollama_llm_server_repository`: `it_should_fail_on_an_answer_that_is_not_ollamas`. Verify: test red, then green.
- [x] 3.6 `sqlite_video_summary_repository`: `it_should_save_and_find_a_summary`. Verify: test red, then green.
- [x] 3.7 `sqlite_video_summary_repository`: `it_should_replace_a_videos_summary`. Verify: test red, then green.
- [x] 3.8 `sqlite_video_summary_repository`: `it_should_delete_a_summary`. Verify: test red, then green.
- [x] 3.9 `sqlite_video_repository`: `it_should_read_missing_summary_state_for_a_new_video`. Verify: test red, then green.
- [x] 3.10 `sqlite_video_repository`: `it_should_update_only_the_summary_fields`. Verify: test red, then green.
- [x] 3.11 `sqlite_migrations`: `it_should_start_stored_videos_with_a_missing_summary`. Verify: test red, then green.

## 4. Verification

- [x] 4.1 Update `doc/ARCHITECTURE.md` (summary step in the walkthrough) and `doc/AI.md` (generation behaviour, native `/api/chat`). Verify: both describe the implemented flow.
- [x] 4.2 Run `scripts/run-tests.sh --only web --only rust`. Verify: fmt, clippy and tests pass.
- [x] 4.3 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, enable AI and summaries against a reachable Ollama via `PATCH /api/preferences`, add a short channel or playlist with `summaries_enabled` `true`, wait for download and subtitles. Verify: `/api/tasks` and the Tasks page (Playwright) show `Summarizing <title>`, and `video_summaries` holds one row with the video `GENERATED`. Stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.4 Manual (human): read a generated summary for a video with chapters and a long one over the budget, on the real `qwen3:4b` Ollama server, and judge the quality. Verify: both are 3 to 5 sentences in the video's language, with no reasoning leaked.

## 5. Audit fixes

Behaviour tasks are TDD like section 2. Refactor tasks change no behaviour: Verify: `cargo test --locked` passes before and after. Commit the uncommitted audit tests and cleanups first.

- [x] 5.1 Commit the audit's added tests and cleanups as they stand. Verify: `cargo test --locked` passes; `git status` clean.
- [x] 5.2 `ollama_llm_server_repository`: `it_should_post_the_chat_with_its_options` gains `truncate: false` and `shift: false`. Verify: test red, then green.
- [x] 5.3 `ollama_llm_server_repository`: `it_should_report_a_refusal_on_a_bad_request` — a reply with Ollama's message as `refusal`. Verify: test red, then green.
- [x] 5.4 `ollama_llm_server_repository`: `it_should_fail_on_an_error_status` — 500 and 404 are `Err`. Verify: test red, then green.
- [x] 5.5 Remove `LlmReply::is_incomplete` and `TRUNCATION_MARGIN`, the false `prompt_eval_count` comment, `it_should_count_a_failed_attempt_when_the_input_was_truncated`; fix `evals/summaries/eval.py` (send `truncate: false`, drop the `truncated` flag). Verify: `cargo test --locked` passes.
- [x] 5.6 `caption_language`: `it_should_estimate_japanese_chinese_and_korean_at_fewer_characters_per_token`. Verify: test red, then green.
- [x] 5.7 `summarize_video_task`: `it_should_fail_the_task_when_the_answer_stopped_at_the_limit` — handler returns `Err`, `summarize(video, is_last_attempt)`. Verify: test red, then green.
- [x] 5.8 `summarize_video_task`: `it_should_become_unavailable_when_rejected_on_the_last_attempt`. Verify: test red, then green.
- [x] 5.9 `summarize_video_task`: `it_should_fail_the_task_and_stay_missing_on_a_server_fault` — also on the last attempt. Verify: test red, then green.
- [x] 5.10 `summarize_video_task`: `it_should_fail_the_task_when_ollama_refuses_the_prompt`. Verify: test red, then green.
- [x] 5.11 Remove the attempt counting: `summary_checks`, `record_summary_failure`, the two `summary_*` columns from migration `0023`, `is_due` from the summary step in `VideoSyncPlan`, `it_should_space_summary_retries_by_a_day`, and `sqlite_video_repository` mappings. Verify: `cargo test --locked` passes.
- [x] 5.12 `summarize_video_task` (logging): failures log at error with the specific reason, per `it_should_log_why_a_generation_was_skipped_or_failed`. Verify: test red, then green.
- [x] 5.13 `sqlite_video_repository`: `it_should_reset_unavailable_summaries_to_missing`. Verify: test red, then green.
- [x] 5.14 `preferences_updater` via `http/preferences`: `it_should_reset_unavailable_summaries_when_the_summary_settings_change` — table of the URL and each setting. Verify: test red, then green.
- [x] 5.15 `preferences_updater`: `it_should_keep_unavailable_summaries_on_an_unrelated_change`, including saving the same values. Verify: test red, then green.
- [x] 5.16 `summarize_video_task`: `it_should_reduce_answers_in_groups_when_they_do_not_fit` — group calls without chapters, last call with them. Verify: test red, then green.
- [x] 5.17 `summarize_video_task`: `it_should_keep_reducing_until_the_answers_fit`, including a budget smaller than two answers, which still ends. Verify: test red, then green.
- [x] 5.18 `summarize_video_task`: `it_should_list_a_chapter_starting_on_the_parts_last_line` — `<=` in `part_chapter_block`. Verify: test red, then green.
- [x] 5.19 Refactor: lazy `summaries_wanted` in `VideoReconciler`, so a video that can't be summarized reads no preferences. Verify: `cargo test --locked` passes.
- [x] 5.20 Refactor: `TimedLine` to `timed_line.rs`, `SummaryMessages` to `summary_messages.rs`, re-exported. Verify: `cargo test --locked` passes.
- [x] 5.21 Refactor: `cut_by_size` keeps a running size and uses a fold. Verify: `cargo test --locked` passes.
- [x] 5.22 Refactor: rename the tests per design.md's Test Plan. Verify: `cargo test --locked` passes.
- [x] 5.23 Refactor tests: remove `Scene`/`BuiltScene` from `summarize_video_task.rs` and `summaries_scheduled` from `sync_video_task.rs`, per `/rust-architect`; assert whole values; move `it_should_not_queue_a_second_summary` to `sqlite_task_repository`. Verify: `cargo test --locked` passes.
- [x] 5.24 Docs: `doc/AI.md` (the backlog wording, retries, Unavailable reset, `truncate: false`, generation time) and `doc/ARCHITECTURE.md`. Verify: both describe the implemented flow.
- [x] 5.25 Run `scripts/run-tests.sh --only web --only rust`. Verify: all pass.
- [x] 5.26 Manual (agent): against real Ollama, an oversized prompt fails with Ollama's refusal logged at error and a long enough video summarizes end to end. Verify: observed in the logs.
