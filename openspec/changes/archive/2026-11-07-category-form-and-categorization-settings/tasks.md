## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md across Rust and web, wired end-to-end: `CategorizationSettings` fields and `categorization_default()`s, `PreferencesChange`, `NewCategory`, `CategoryCreator::new` with its new dependencies in `serve.rs` and existing tests, `CreateCategoryError::MemberNotTracked` mapped to 400, `CreateCategoryRequest` fields ignored for now, `members_from` signature, web types and `helpers.tsx` builders, `CategorySettingsDialog` renamed to `CategoryDialog` with the same behaviour, `CategoryPromptHelp` and `categoryForm.ts` as stubs. Bodies are trivial, no `todo!()`. Done when `cargo build` and `scripts/run-tests.sh --only web --only rust` pass with no new behaviour and no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `http/preferences`: `it_should_ship_the_categorization_prompt_v2`: `SHIPPED`, `preferences/mod.rs:238` and `helpers.tsx` use `v2.md`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.2 `http/preferences`: `it_should_return_the_categorization_generation_defaults_if_nothing_saved`: 4096, 0, thinking on. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.3 `http/preferences`: `it_should_save_the_categorization_generation_settings`: returned and stored, summary values unchanged. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.4 `http/preferences`: `it_should_fail_if_the_categorization_context_length_is_out_of_range`: 400 with its message. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.5 `http/preferences`: `it_should_fail_if_the_categorization_temperature_is_out_of_range`: 400 with its message. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.6 `http/categories`: `it_should_create_a_category_with_a_prompt`: response and stored prompt, trimmed. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.7 `http/categories`: `it_should_create_a_category_with_members`: response holds the lists, members stored. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.8 `http/categories`: `it_should_put_the_videos_of_its_members_in_a_created_category`: stored videos belong to it, events announced. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.9 `http/categories`: `it_should_create_a_category_without_a_prompt_if_blank`: `""` stores `None`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.10 `http/categories`: `it_should_fail_to_create_a_category_with_an_untracked_member`: 400, nothing stored. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.11 `http/categories`: `it_should_fail_to_create_a_category_if_the_prompt_is_too_long`: 400, nothing stored. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.12 `Web aiSettingsForm.test.ts`: `changedFields` includes the new fields only when changed, drops `categorization_thinking` when that model can't think, `hasInvalidNumber` covers both new numbers. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.13 `Web AiSettingsView.test.tsx`: categorization Advanced shows `4096`, `0`, thinking on and the prompt. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.14 `Web AiSettingsView.test.tsx`: saves only the changed advanced field, independently of summaries. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.15 `Web AiSettingsView.test.tsx`: a model that can't think hides only that section's switch. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.16 `Web AiSettingsView.test.tsx`: an emptied categorization number blocks saving with the message. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.17 `Web categoryForm.test.ts`: `changedDetails` returns only what differs, no prompt without `withPrompt`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.18 `Web CategoryDialog.test.tsx`: create shows name, color, prompt and chips in order, name focused, next free color preselected. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.19 `Web CategoryDialog.test.tsx`: create sends everything in one `POST` and navigates with no dialog open; `AddCategoryDialog` removed and `App.tsx` switched. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.20 `Web CategoryDialog.test.tsx`: create without categorization has no prompt field and sends no `ai_prompt`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.21 `Web CategoryDialog.test.tsx`: create failing keeps everything and shows the error; a blank name disables it. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.22 `Web CategoryDialog.test.tsx`: create dismissal asks to discard only with something entered. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.23 `Web CategoryDialog.test.tsx`: edit prefills name and color; saving sends one update and no members. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.24 `Web CategoryDialog.test.tsx`: edit with a taken name keeps the dialog and doesn't send members. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.25 `Web CategoryDialog.test.tsx`: the new partial failure and discard messages. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.26 `Web CategoryDialog.test.tsx`: Enter in the search with several matches neither toggles nor saves; Enter in "Name" saves. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.27 `Web CategoryPromptHelp.test.tsx`: opening shows the good and bad examples and tips. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.28 `Web CategoryDialog.test.tsx`: closing the guide with Escape keeps the dialog and the staged change. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.29 `Web CategoryDetail.test.tsx`: no "Change color" action, and the color changes through "Edit settings"; `CategoryColorDialog` and `draftColor` removed. Verify: the test goes red on an assertion, then green with the full suite.

## 3. Infrastructure adapters (TDD)

- [x] 3.1 `sqlite_preferences_repository`: `it_should_read_the_categorization_defaults_if_stored_preferences_predate_them`: 4096, 0, true, not the summary values. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 3.2 `sqlite_preferences_repository`: `it_should_read_invalid_categorization_settings_as_their_defaults`: out-of-range stored values fall back to 4096 and 0. Verify: the test goes red on an assertion, then green with the full suite.

## 4. Verification

- [x] 4.1 Update `doc/AI.md`, `doc/CATEGORIES.md` and `prompts/categorization/README.md` (v2 shipped, still under evaluation; new settings; one dialog). Verify: no reference to `categorization/v1.md` as the default or to "Change color" remains (`grep`).
- [x] 4.2 Update `e2e-tests/tests/categories.spec.js` and `aiSettings.spec.js`, then run `scripts/run-tests.sh --only e2e`. Verify: the suite passes, with the create flow going through one dialog and `POST /api/categories` covering prompt and sources.
- [x] 4.3 Run `scripts/run-tests.sh --only web --only rust`. Verify: fmt, clippy and tests pass.
- [x] 4.4 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, seed preferences (AI, categorization on) and a channel via the HTTP API, then with the Playwright MCP browser: create a category with name, color, prompt and a source and assert the single `POST` and the resulting view; edit it and assert one `PATCH`; open the "?" guide; open the AI page and assert `4096`, `0`, thinking on and the v2 prompt under Advanced; repeat the dialog at phone width with a screenshot. Stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.5 Manual (human): on a real phone, check the keyboard with "Name" focused, tapping the "?" guide, and that the dialog at 48rem feels right on desktop with many sources.
