## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md across Rust and web, wired end-to-end: migration registered in `sqlite_migrations.rs`, repositories persisting nothing new yet, DTOs returning fixed values, `SummariesField` replaced by `SourceAiSection` rendering only "Summarize videos", `ManageSourcesDialog` file renamed to `CategorySettingsDialog` with the same text, `prompts/categorization/v1.md` written, web types and `helpers.tsx` builders updated, and the e2e helpers and specs sending `categorization_enabled: false`. Bodies are trivial, no `todo!()`. Done when `cargo build` and `scripts/run-tests.sh --only web --only rust` pass with no new behaviour and no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `http/preferences`: `it_should_return_the_categorization_defaults_if_nothing_saved`: off, `qwen3:4b`, shipped prompt, `is_default` true. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.2 `http/preferences`: `it_should_save_the_categorization_settings`: patch returns and stores enabled, model, prompt, `is_default` false. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.3 `http/preferences`: `it_should_go_back_to_the_shipped_categorization_prompt`: `null` drops the saved prompt. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.4 `http/preferences`: `it_should_fail_if_the_categorization_model_is_blank`: 400 with its message. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.5 `http/preferences`: `it_should_fail_if_the_categorization_prompt_is_blank`: 400 with its message. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.6 `http/preferences`: `it_should_keep_unavailable_summaries_when_the_categorization_settings_change`: summary stays `Unavailable`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.7 `http/categories`: `it_should_set_the_ai_prompt_of_a_category`: stored trimmed; name, color, members kept. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.8 `http/categories`: `it_should_remove_the_ai_prompt_of_a_category`: `""` stores `None`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.9 `http/categories`: `it_should_keep_the_ai_prompt_when_renaming_a_category`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.10 `http/categories`: `it_should_fail_if_the_ai_prompt_is_too_long`: 2001 characters → 400, unchanged. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.11 `http/categories`: `it_should_list_categories_with_their_ai_prompt`: value and `null`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.12 `http/categories`: `it_should_fail_to_update_a_category_with_nothing_to_change`: new message (existing test renamed). Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.13 `http/channels, then http/playlists`: `it_should_create_a_channel_with_categorization_enabled` / `…_playlist_…`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.14 `http/channels, then http/playlists`: `it_should_fail_to_create_a_channel_with_categorization_if_unavailable` / `…_playlist_…`: AI, categories or categorization off → 409, nothing stored, no lookup. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.15 `http/channels, then http/playlists`: `it_should_fail_to_create_if_categorization_missing` (both). Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.16 `http/channels, then http/playlists`: `it_should_keep_the_existing_source_when_categorization_differs_on_create` (both): duplicate returns the stored `false`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.17 `http/channels, then http/playlists`: `it_should_enable_categorization_on_a_channel` / `…_playlist`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.18 `http/channels, then http/playlists`: `it_should_fail_to_enable_categorization_on_a_channel_if_unavailable` / `…_playlist_…`: other fields unchanged. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.19 `http/channels, then http/playlists`: `it_should_disable_categorization_on_a_channel_while_ai_is_off` / `…_playlist_…`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.20 `http/channels, then http/playlists`: `it_should_list_channels_with_their_categorization_setting` / `playlists`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.21 `Web AiSettingsView.test.tsx`: shows the Categorization section disabled on a fresh install. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.22 `Web AiSettingsView.test.tsx`: hides the Categorization section while categories are off. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.23 `Web AiSettingsView.test.tsx`: shows the shipped prompt under Advanced with `Using the default prompt`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.24 `Web AiSettingsView.test.tsx`: saves only the changed categorization fields. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.25 `Web AiSettingsView.test.tsx`: resets a saved categorization prompt. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.26 `Web AiSettingsView.test.tsx`: lists the server's models in the categorization Model independently of summaries. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.27 `Web aiSettingsForm.test.ts, preferences.test.ts`: `changedFields` includes categorization fields only when changed. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.28 `Web aiSettingsForm.test.ts, preferences.test.ts`: `categorizationAvailable` needs AI, categories and categorization. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.29 `Web source dialogs`: `AddChannelDialog`: "Summarize videos" and "Categorize videos" sit in the "AI" group. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.30 `Web source dialogs`: `AddChannelDialog`: creates with `categorization_enabled` false by default, true when checked. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.31 `Web source dialogs`: `AddPlaylistDialog`: same as 30. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.32 `Web source dialogs`: `AddChannelDialog`: disabled with a link to AI settings while unavailable; hidden with categories off. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.33 `Web source dialogs`: `EditChannelDialog`: prefilled, sends only `categorization_enabled` when changed. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.34 `Web source dialogs`: `EditChannelDialog`: stays enabled to turn off while AI is off. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.35 `Web source dialogs`: `EditPlaylistDialog`: same as 33. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.36 `Web category settings`: `SidebarCategories`: row menu offers "Edit settings", opening "Edit Science settings". Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.37 `Web category settings`: `CategoryDetail`: header action "Edit settings"; arrival with `CategorySettingsState` opens it once. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.38 `Web category settings`: `AddCategoryDialog`: creating opens "Edit Chess settings". Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.39 `Web category settings`: `CategorySettingsDialog`: shows "AI prompt" prefilled while available. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.40 `Web category settings`: `CategorySettingsDialog`: hides it otherwise and saves sources without `ai_prompt`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.41 `Web category settings`: `CategorySettingsDialog`: saving only the prompt doesn't send members. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.42 `Web category settings`: `CategorySettingsDialog`: saving both sends both, then closes. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.43 `Web category settings`: `CategorySettingsDialog`: emptying the prompt sends `ai_prompt: ""`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.44 `Web category settings`: `CategorySettingsDialog`: an edited prompt enables Save and asks to discard on cancel. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.45 `Web category settings`: `CategorySettingsDialog`: a failed save stays open and keeps the text. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.46 `Web category settings`: `CategorySettingsDialog`: a prompt saved elsewhere is left alone when saving the sources. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.47 `Web category settings`: `CategorySettingsDialog`: a hidden prompt changed elsewhere stages nothing. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.48 `Web category settings`: `CategorySettingsDialog`: an edited prompt is left out once its field is hidden. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.49 `Web category settings`: `CategorySettingsDialog`: a pasted prompt is sent whole, for the daemon to judge its length. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.50 `Web category settings`: `CategorySettingsDialog`: says the prompt was saved when only the sources failed, and retries the sources. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 2.51 `Web category settings`: `CategorySettingsDialog`: says the prompt stays saved when discarding after only the sources failed. Verify: the test goes red on an assertion, then green with the full suite.

## 3. Infrastructure adapters (TDD)

- [x] 3.1 `SqlitePreferencesRepository`: `it_should_round_trip_the_categorization_settings`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 3.2 `SqlitePreferencesRepository`: `it_should_read_missing_categorization_keys_as_defaults`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 3.3 `SqliteCategoryRepository`: `it_should_round_trip_the_ai_prompt`: insert, update, clear, `list`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 3.4 `SqliteChannelRepository, SqlitePlaylistRepository`: `it_should_round_trip_the_channels_categorization_setting`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 3.5 `SqliteChannelRepository, SqlitePlaylistRepository`: `it_should_round_trip_the_playlists_categorization_setting`. Verify: the test goes red on an assertion, then green with the full suite.
- [x] 3.6 `sqlite_migrations`: `it_should_default_existing_sources_and_categories_after_the_ai_categorization_migration`: `false` and `NULL`. Verify: the test goes red on an assertion, then green with the full suite.

## 4. Verification

- [x] 4.1 Update `doc/AI.md` and `doc/CATEGORIES.md` (and `doc/ARCHITECTURE.md` if it lists these settings); verify `grep -rn "Manage sources" doc web/src` finds nothing.
- [x] 4.2 Run `scripts/run-tests.sh --only web --only rust`; verify fmt, clippy and tests pass.
- [x] 4.3 Run `scripts/run-tests.sh --only e2e`, since the e2e helpers and specs changed; verify it passes.
- [x] 4.4 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, seed a category and a channel through the HTTP API, then with the Playwright MCP browser: on `/ai` turn on AI and categorization, pick a model, edit and save the prompt, and reset it; in the category's "Edit settings" save an AI prompt and confirm `GET /api/categories` returns it; in the channel's "Edit settings" check "Categorize videos" under "AI" and confirm `GET /api/channels` returns `categorization_enabled: true`; turn categorization off and confirm "AI prompt" is gone while the saved prompt stays in the API. Take a screenshot of the dialog at phone width and check the chips still scroll. Stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
