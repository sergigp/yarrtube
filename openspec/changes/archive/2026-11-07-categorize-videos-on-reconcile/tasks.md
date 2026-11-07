# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures) across all layers: migration `0025_video_category_verdicts.sql`, `VideoCategoryVerdict`/`Verdict`/`VerdictAuthor`, `pending_categories`, `CategorizationQuestion`/`CategorizationAnswer`, `LlmChat.format` (summarizer passes `None`), `VideoOwner.categorization_enabled`/`source_name`, `Video::is_categorizable`, `VideoSyncStep::Categorize`, `VideoSyncPlan::for_video`'s new parameter, `Task::CategorizeVideo` (type, lane, key, decode), `Preferences::changes_categorization`, `VideoCategorizer` + `VideoCategorizerApi`, `VideoCategoryAssignerApi::apply_verdict`, `VideoCategoryVerdictRepository` + SQLite adapter (wired into the assigner, both video reconcilers, `ChannelDeleter` and `PlaylistDeleter`), `CategorizeVideoTask` registered as `categorize_video`, everything constructed in `serve.rs`/`infrastructure_container.rs`, the task type list in `serve.rs`, `task_view_searcher` and `web/src/lib/tasks.ts` entries. Bodies return trivial values (`Ok(())`, empty `Vec`, `None`, `false`), shipped prompt switched to `v2.md`. Done when `cargo build` succeeds and all existing tests pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_give_a_video_its_source_categories_plus_its_yes_verdicts`: union in `assign_video`
- [x] 2.2 `it_should_give_a_video_of_an_uncategorized_source_its_yes_verdicts`: no early return for a source in no category
- [x] 2.3 `it_should_schedule_a_categorization_for_a_summarized_video_with_pending_categories`: `categorization_wanted` + `Categorize` step
- [x] 2.4 `it_should_not_schedule_a_categorization_without_a_generated_summary`
- [x] 2.5 `it_should_not_schedule_a_categorization_when_the_source_has_categorization_off`
- [x] 2.6 `it_should_not_schedule_a_categorization_when_ai_categorization_is_off`
- [x] 2.7 `it_should_not_schedule_a_categorization_when_every_category_has_a_verdict`
- [x] 2.8 `it_should_not_count_a_category_given_by_the_source_or_without_a_prompt_as_pending`: `pending_categories`
- [x] 2.9 `it_should_not_schedule_a_second_categorization_while_one_is_pending`: task dedupe for `categorize_video`
- [x] 2.10 `it_should_store_a_yes_verdict_and_add_the_category_to_the_video`: `categorize` + `apply_verdict` + event
- [x] 2.11 `it_should_store_a_no_verdict_without_changing_the_video_categories`
- [x] 2.12 `it_should_send_the_video_details_and_category_prompt_with_the_categorization_settings`: `CategorizationQuestion::user_message`/`chat`
- [x] 2.13 `it_should_read_the_answer_after_leaked_thinking`: `CategorizationAnswer::parse`
- [x] 2.14 `it_should_only_ask_about_categories_pending_when_it_runs`
- [x] 2.15 `it_should_do_nothing_when_the_conditions_no_longer_hold`: `prepare` skips
- [x] 2.16 `it_should_not_store_a_verdict_for_a_category_deleted_during_its_call`: existence check under the lock
- [x] 2.17 `it_should_not_store_a_verdict_for_a_video_removed_during_its_call`: video existence check in `apply_verdict` under the lock
- [x] 2.18 `it_should_fail_after_trying_every_category_when_a_call_is_rejected`
- [x] 2.19 `it_should_retry_only_the_categories_still_pending`
- [x] 2.20 `it_should_mark_rejected_categories_unavailable_on_the_last_attempt`
- [x] 2.21 `it_should_complete_without_verdicts_when_ollama_is_unreachable`
- [x] 2.22 `it_should_log_each_verdict`
- [x] 2.23 `it_should_delete_a_categorys_verdicts_with_it`: `CategoryDeleter`
- [x] 2.24 `it_should_keep_yes_verdict_categories_when_a_source_leaves_a_category`: `reassign_category` union
- [x] 2.25 `it_should_make_unavailable_verdicts_pending_when_a_category_prompt_changes`: `CategoryUpdater`
- [x] 2.26 `it_should_keep_verdicts_when_a_category_prompt_is_edited_or_cleared`
- [x] 2.27 `it_should_delete_unavailable_verdicts_when_categorization_settings_change`: `PreferencesUpdater`
- [x] 2.28 `it_should_keep_unavailable_verdicts_when_the_same_settings_are_saved`
- [x] 2.29 `it_should_delete_a_removed_videos_verdicts`: `ChannelVideoReconciler::remove_videos_not_in`
- [x] 2.30 `it_should_delete_a_removed_playlist_videos_verdicts`: `PlaylistVideoReconciler::remove_videos_not_in`
- [x] 2.31 `it_should_delete_the_verdicts_of_a_deleted_channels_videos`: `ChannelDeleter`
- [x] 2.32 `it_should_delete_the_verdicts_of_a_deleted_playlists_videos`: `PlaylistDeleter`
- [x] 2.33 `labels a categorize_video task and files it under AI` (web `tasks.test.ts`)
- [x] 2.34 Update `doc/AI.md` and `doc/CATEGORIES.md` (when a video is categorized, verdicts are final, summaries needed, toggle-off keeps categories) and `prompts/categorization/README.md` (v2 shipped); verify the docs match the specs
- [x] 2.35 `it_should_not_mark_a_category_unavailable_when_the_settings_changed_during_its_attempt`: `give_up` via `apply_verdict_if` + `unchanged_since`; updaters delete under the category lock
- [x] 2.36 `it_should_not_keep_a_verdict_for_a_video_removed_while_it_was_stored`: `forget_if_removed` in `apply_verdict`
- [x] 2.37 `it_should_keep_unavailable_verdicts_when_a_category_prompt_is_saved_unchanged`, `it_should_list_a_categorize_video_task_with_video_and_channel_names`
- [x] 2.38 Value object tests: `Verdict`, `VerdictAuthor`, `CategorizationAnswer::parse`
- [x] 2.39 `doc/CATEGORIES.md`: a video is categorized from its next sync after its summary, at the latest its source's next reconcile pass

## 3. Infrastructure adapters (TDD)

`SqliteVideoCategoryVerdictRepository`

- [x] 3.1 `it_should_save_and_find_verdicts_of_several_videos`
- [x] 3.2 `it_should_replace_a_pairs_verdict`
- [x] 3.3 `it_should_delete_a_videos_verdicts`
- [x] 3.4 `it_should_delete_a_categorys_verdicts`
- [x] 3.5 `it_should_delete_only_unavailable_verdicts`
- [x] 3.6 `it_should_delete_the_unavailable_verdicts_of_one_category`

`OllamaLlmServerRepository`

- [x] 3.7 `it_should_send_the_response_schema_as_format`

## 4. Verification

- [x] 4.1 `scripts/run-tests.sh --only web --only rust` passes (fmt, clippy, tests)
- [x] 4.2 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos` with Ollama reachable, seed via the HTTP API a channel with summaries and categorization on, a category with an AI prompt and AI/categories/categorization on; once a video has a summary, assert a `categorize_video` task runs (tasks API), the verdict appears in `video_category_verdicts`, a yes adds the category (category videos API, tasks view label "Categorizing …" via Playwright), and a second reconcile schedules no new `categorize_video`; then stop the server and delete `yarrtube.sqlite3` and the fresh videos dir
- [ ] 4.3 Manual (human): on the NAS with real Ollama, check verdict quality and timing on a real channel, and that Plex labels follow the new categories
