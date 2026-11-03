# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures) across Rust and web, wired end-to-end: `PATCH /api/preferences` replaces `PUT`, `GET /api/ollama` route, `LlmServerSearcher` in `ApiServices` + `serve.rs`, `OllamaLlmServerRepository` in the container, `/ai` route + "AI" menu entry rendering an empty `AiSettingsView`, web client/queries switched to `patchPreferences`, `aPreferences`/`anOllamaServer` builders, `Dockerfile` `COPY prompts ./prompts` + `.dockerignore` `!prompts/**/*.md`, e2e fixture reset via `PATCH`. Bodies trivial (`Ok(LlmServer { version: String::new(), models: vec![] })`, `apply` returns `self`, VOs accept anything, `shipped()` returns the embedded file). Done when `cargo build`, `cargo test --locked` and `npm run check` pass with no new tests.

## 2. Behaviour (TDD)

Preferences handler (acceptance, real SQLite)
- [x] 2.1 `it_should_return_the_ai_defaults_if_nothing_saved` — GET returns AI off, URL null, qwen3:4b/12288/0.3/thinking on, shipped prompt, `summaries_prompt_is_default` true.
- [x] 2.2 `it_should_change_only_the_preferences_sent` — patch keeps unsent stored values (port/replace the old full-PUT tests).
- [x] 2.3 `it_should_keep_everything_on_an_empty_patch`.
- [x] 2.4 `it_should_save_the_ollama_url_without_trailing_slash`.
- [x] 2.5 `it_should_unset_the_ollama_url_on_an_empty_string`.
- [x] 2.6 `it_should_enable_ai_with_a_url`.
- [x] 2.7 `it_should_fail_to_enable_ai_without_a_url` — 400 `Ollama URL must be set to enable AI`, stored unchanged.
- [x] 2.8 `it_should_fail_to_unset_the_url_while_ai_is_on`.
- [x] 2.9 `it_should_save_the_summary_settings` — model, context length, temperature, thinking.
- [x] 2.10 `it_should_fail_on_an_out_of_range_temperature_and_save_nothing` — valid `categories_enabled` sent alongside stays unchanged.
- [x] 2.11 `it_should_save_an_edited_prompt` — `summaries_prompt_is_default` false.
- [x] 2.12 `it_should_not_store_the_prompt_when_saving_other_preferences`.
- [x] 2.13 `it_should_return_to_the_shipped_prompt_on_null`.
- [x] 2.14 `it_should_fail_on_an_invalid_prompt` — `Prompt is missing the "map user" section`.
- [x] 2.15 `it_should_save_the_theme` and `it_should_fail_on_an_unknown_theme` ported to PATCH.

Ollama handler (acceptance, mockito Ollama)
- [x] 2.16 `it_should_return_the_server_version_and_models_sorted_by_name` — each with its `thinking` capability.
- [x] 2.17 `it_should_return_no_models_if_none_installed`.
- [x] 2.18 `it_should_fail_if_the_url_is_missing` — 400 `Ollama URL must be stated (missing)`.
- [x] 2.19 `it_should_fail_if_the_url_is_not_http` — 400 `Ollama URL must be an http(s) URL (got "…")`.
- [x] 2.20 `it_should_fail_with_bad_gateway_if_ollama_is_unreachable` — 502, `Could not reach Ollama at <url>: …`.

Value objects
- [x] 2.21 `OllamaUrl` tests: http/https accepted, trailing slash stripped, schemeless/ftp/empty rejected with the spec message.
- [x] 2.22 `SummaryModel` tests: blank and > 200 chars rejected with the spec message.
- [x] 2.23 `ContextLength` tests: 1024 and 131072 accepted, 1023 and 131073 rejected with the spec message.
- [x] 2.24 `Temperature` tests: 0 and 2 accepted, −0.1, 2.1 and NaN rejected with the spec message.
- [x] 2.25 `SummaryPrompt` tests, one cycle each: shipped prompt valid; missing section; unknown section; repeated section; empty section; disallowed variable message; note before first section ignored; non-variable braces ignored.

Web (Vitest)
- [x] 2.26 `aiSettingsForm.test.ts` — `changedFields` sends only changed keys, omits an untouched prompt, empty URL → `""`.
- [x] 2.27 `promptGuide.test.ts` — every section and every `{var}` of the shipped prompt (`?raw` import) is in the guide under its section, each with a description.
- [x] 2.28 `AiSettingsView.test.tsx` — fresh defaults with summary fields disabled; enabling AI enables them.
- [x] 2.29 `AiSettingsView.test.tsx` — Save sends only changed fields and shows "Saved"; a 400 shows the daemon message and keeps the form.
- [x] 2.30 `AiSettingsView.test.tsx` — Test connection shows `Connected to Ollama 0.40.13, 2 models`, or the daemon error; disabled with an empty URL.
- [x] 2.31 `AiSettingsView.test.tsx` — model list from the saved URL, `(not installed)` entry, text field when the check fails.
- [x] 2.32 `AiSettingsView.test.tsx` — Thinking switch shown for a thinking model and when models are unknown; for a non-thinking model replaced by `<model> doesn't support thinking` and `summaries_thinking` not sent; help text.
- [x] 2.33 `AiSettingsView.test.tsx` — `Using the default prompt` note; Reset sends `summaries_prompt: null`; prompt guide shown.
- [x] 2.34 `SettingsMenu.test.tsx` — "AI" between "Preferences" and "Report a bug", navigates to `/ai`.
- [x] 2.35 `PreferencesView.test.tsx`, `ThemeToggle.test.tsx` — expect `PATCH /api/preferences` with only the changed preference.

## 3. Infrastructure adapters (TDD)

`sqlite_preferences_repository`
- [x] 3.1 `it_should_find_saved_ai_preferences` — round trip with URL and prompt.
- [x] 3.2 `it_should_read_missing_ai_keys_as_defaults` — row in the pre-AI JSON shape.
- [x] 3.3 `it_should_find_no_prompt_if_saved_without_one`.

`ollama_llm_server_repository` (mockito)
- [x] 3.4 `it_should_read_the_version_and_models`.
- [x] 3.5 `it_should_read_whether_each_model_can_think` — from `/api/show` `capabilities`.
- [x] 3.6 `it_should_fail_if_the_version_endpoint_is_not_json`.
- [x] 3.7 `it_should_fail_if_the_server_replies_with_an_error`.

## 4. Verification

- [x] 4.1 E2E: `e2e-tests/ollama-stub.js` (`/api/version`, `/api/tags`, `/api/show`) started like the Plex stub; `aiSettings.spec.js` covers Settings → AI, Test connection, enable AI + summaries, pick model, Save, reload shows saved values, Reset prompt. Passes in `scripts/run-tests.sh`.
- [x] 4.2 `docker build .` succeeds (prompt file reaches the build context).
- [x] 4.3 Update `doc/AI.md` (settings live in the app; prompt default = shipped v3, edits stored in preferences).
- [x] 4.4 `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `scripts/run-tests.sh` (no flags) all pass.
- [x] 4.5 Manual (agent): `scripts/run-local.sh --port <free port> --fresh-videos`; with Playwright MCP open Settings → AI, test against the user's real Ollama (or a stub), enable AI + summaries, pick a model, edit a prompt rule, Save, reload and assert values persisted; break a section and assert the error; Reset and assert `Using the default prompt`; screenshot; stop server, delete `yarrtube.sqlite3` and the fresh videos dir.
- [x] 4.6 Manual (human): visual judgement of the AI page layout (light/dark, mobile width) and the prompt editor's usability with the full ~4 KB prompt.

## Workflow follow-up

- Next change: generate, store and show summaries using these settings; send `think` only for models whose capabilities include `thinking`, always strip up to `</think>`.
