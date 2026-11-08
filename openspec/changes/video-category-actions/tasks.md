# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures) across all layers: `VideoCategoryChoice`/`CategoryOrigin`, `EditVideoCategoriesError`, `VideoCategoryVerdict::by_user`, `VideoCategoryChoiceSearcher`, `VideoCategoryEditor`, `VideoCategoryAssignerApi::recompute`, the new `VideoCategoryVerdictRepository` methods, `CategoryView.ai_verdict_count`, `HomeVideoView.record_id` and `record_id` on the three video response DTOs (filled with an empty string for now), the six HTTP handlers and DTOs with their routes registered, services constructed in `serve.rs`; web types (`record_id` on `Video`/`HomeVideo`), client calls taking the record id, query hook, fixture builders, an empty `VideoCategoriesDialog`, `VideoActionsMenu`'s `recordId`/`sourceName` props and `VideoListPane`'s `sourceNameOf`, passed by `VideoGrid`, `ChannelDetail`, `PlaylistDetail`, `CategoryPlayer`. Bodies return trivial values (`Ok(())`, empty `Vec`, `0`, `StatusCode::NO_CONTENT`). Done when `cargo build` and `npm run typecheck` succeed and all existing tests pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_include_the_record_id_when_listing_playlist_videos`
- [x] 2.2 `it_should_include_the_record_id_when_listing_category_videos`
- [x] 2.3 `it_should_include_the_record_id_on_home`: `HomeVideoView.record_id`
- [x] 2.4 `it_should_list_a_videos_categories_with_where_each_comes_from`: `VideoCategoryChoice::list` + searcher
- [x] 2.5 `it_should_report_the_source_for_a_member_category_even_with_a_verdict`
- [x] 2.6 `it_should_return_not_found_for_the_categories_of_an_unknown_video`
- [x] 2.7 `it_should_add_a_category_by_hand_replacing_an_ai_no`: `VideoCategoryEditor::set`
- [x] 2.8 `it_should_remove_an_ai_category_by_hand`
- [x] 2.9 `it_should_reject_setting_a_category_given_by_the_source`
- [x] 2.10 `it_should_reject_a_missing_or_non_boolean_belongs`
- [x] 2.11 `it_should_return_not_found_for_an_unknown_video_or_category_when_setting`
- [x] 2.12 `it_should_rerun_a_videos_ai_categorization`: `rerun_video` + `recompute` + `SyncVideo`
- [x] 2.13 `it_should_return_not_found_when_rerunning_an_unknown_video`
- [x] 2.14 `it_should_not_evaluate_a_pair_with_a_user_verdict`
- [x] 2.15 `it_should_clear_a_channels_ai_verdicts_keeping_user_ones`: `rerun_source` + `ReconcileChannel`
- [x] 2.16 `it_should_clear_a_playlists_ai_verdicts_keeping_user_ones`
- [x] 2.17 `it_should_return_not_found_when_clearing_an_unknown_source`
- [x] 2.18 `it_should_rerun_a_categorys_ai_verdicts`: `rerun_category`
- [x] 2.19 `it_should_return_not_found_when_rerunning_an_unknown_category`
- [x] 2.20 `it_should_count_ai_verdicts_per_category`: `CategoryViewSearcher`
- [x] 2.21 `VideoCategoriesDialog`: shows locked source chips, AI marks and filled chips
- [x] 2.22 `VideoCategoriesDialog`: sets a category by hand when a chip is clicked
- [x] 2.23 `VideoCategoriesDialog`: shows the error and keeps the chip when setting fails
- [x] 2.24 `VideoCategoriesDialog`: re-runs AI only while AI categorization is available
- [x] 2.25 `VideoActionsMenu`: offers Edit categories only while categories are on
- [x] 2.26 `DetailHeader`: offers Re-run AI categorization or Clear AI categories after confirmation
- [x] 2.27 `CategorySettingsDialog`: re-evaluates N videos after confirmation, hidden at 0
- [x] 2.28 `videoCategories.spec.js` (e2e): add and remove a category by hand on a seeded video; chip and category page follow
- [x] 2.29 Update `doc/CATEGORIES.md` and `doc/AI.md` (editing by hand, locked source chips, re-run per video, source and category, clear when off); verify they match the specs

## 3. Infrastructure adapters (TDD)

`SqliteVideoCategoryVerdictRepository`

- [x] 3.1 `it_should_delete_only_ai_verdicts_of_the_given_videos_returning_them`
- [x] 3.2 `it_should_delete_only_ai_verdicts_of_a_category_returning_their_videos`
- [x] 3.3 `it_should_count_ai_verdicts_per_category`

## 4. Verification

- [x] 4.1 `scripts/run-tests.sh --only web --only rust` passes (fmt, clippy, tests)
- [x] 4.2 `scripts/run-tests.sh --only e2e` passes (new `videoCategories.spec.js`)
- [x] 4.3 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, seed via the HTTP API a channel in "Chess", a category "Sicilian" with a prompt and a video with an AI `yes` verdict for it (insert the verdict in SQLite); with Playwright open the video's "⋮" → "Edit categories…", assert "Chess" is locked with "From …", "Sicilian" has the AI mark, click it and assert the PUT and the outlined chip; use "Clear AI categories" from the channel menu and assert the DELETE and the confirmation; open the category settings and assert "Re-evaluate N videos"; take screenshots; then stop the server and delete `yarrtube.sqlite3` and the fresh videos dir
- [ ] 4.4 Manual (human): judge the dialog's look on desktop and phone widths (chip states readable, AI mark and lock clear), and that Plex labels follow a hand edit

## 5. Audit follow-ups

- [x] 5.1 `it_should_not_override_a_user_verdict_set_during_its_call`, `it_should_not_store_an_answer_to_a_prompt_changed_during_its_call`: `VideoCategorizer::still_pending` guards `evaluate` and `give_up`
- [x] 5.2 `CategorySettingsDialog`: keeps re-evaluate disabled while the prompt has unsaved edits
- [x] 5.3 `it_should_say_whether_the_videos_source_has_categorization_on`: `VideoCategoryChoices`, `categorization_enabled` on `GET /api/videos/{id}/categories` and the PUT response; web type and the dialog fixtures in `web/src/test/helpers.tsx`
- [x] 5.4 `VideoCategoriesDialog`: offers Clear AI categories when the source has categorization off
- [x] 5.5 Update `doc/AI.md` and `doc/CATEGORIES.md`: the per-video button clears when the source is off; an answer in flight never overrides a hand edit or a changed prompt; re-evaluate needs the prompt saved
- [x] 5.6 `scripts/run-tests.sh --only web --only rust` passes

## Workflow follow-up

- Archive `categorize-videos-on-reconcile` before applying this change, so `video-categorization` and the verdict repository exist.
