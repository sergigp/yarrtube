# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures), wired end to end. The pieces:
  - `CategorizationQuestion` over `&[Category]`, with `answers`, `max_output_tokens` and `timeout`.
  - `LlmChat.timeout`, set by the summarizer to 30 min and used by the Ollama adapter instead of `CHAT_TIMEOUT`.
  - `LlmReply.output_tokens`.
  - `ContextLength::auto`.
  - `CategorizationSettings.context_length: Option<ContextLength>` and `effective_context_length`.
  - `PreferencesChange.categorization_context_length: Option<Option<ContextLength>>`.
  - `ResolvedPreferences`, returned by `PreferencesSearcher::get` and `PreferencesUpdater::update`, both given the `CategoryRepository` in the composition root.
  - The DTO: a nullable PATCH field, and `categorization_context_length_is_auto` in the response.
  - `VideoCategorizer::ask`, `fall_back`, `store` and the `Call` enum, with the existing per-category behaviour kept.
  - Web: `types.ts`, the `helpers.tsx` builder and the `aiSettingsForm` auto field, with no UI yet.

  Bodies keep today's behaviour or return trivial values: Auto resolves to 4096, the cap stays 3072, no fallback. Done when `cargo build`, `cargo test --locked` and `npm run check` pass, with existing tests adjusted only for the new fields.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_ask_about_every_pending_category_in_one_call`: 3 pending categories go into one chat listing all three, and 3 verdicts are stored. Rewrite the existing categorize tests to the batch answer shape in the refactor step. Verify with `cargo test categorize_video_task`.
- [x] 2.2 `it_should_send_the_video_details_and_category_prompts_with_the_categorization_settings`: the exact `LlmChat`, with the user message per the spec, the schema keyed by name and a 4,095-token cap for 3 categories.
- [ ] 2.3 `it_should_cap_the_output_at_3072_for_one_category`.
- [ ] 2.4 `it_should_cap_the_output_at_the_context_length`: an 8192 override with 9 categories gives a cap of 8192.
- [ ] 2.5 `it_should_wait_longer_for_a_call_about_more_categories`: 9 categories give a 2,458 s timeout; 1 category gives 30 min.
- [ ] 2.6 `it_should_ask_with_the_auto_context_length`: 9 categories with a prompt give `context_length` 16384 (drives `ContextLength::auto`; add its inline value-object test for 0, 2, 3, 9, 10 and 80 categories).
- [ ] 2.7 `it_should_store_the_usable_answers_of_a_partly_usable_call`: "Sicilian" is stored and "Tech" is asked alone (drives `CategorizationQuestion::answers`; add its inline tests for leaked thinking, an entry without `belongs`, a missing entry and no JSON).
- [ ] 2.8 `it_should_ask_each_category_alone_when_the_call_hits_the_output_limit`: 1 batch call plus N single calls, all stored.
- [ ] 2.9 `it_should_ask_each_category_alone_when_the_call_is_refused`.
- [ ] 2.10 `it_should_not_fall_back_for_a_single_category`: one rejected call, then the task fails.
- [ ] 2.11 `it_should_mark_unavailable_only_what_its_own_call_rejected_on_the_last_attempt`.
- [ ] 2.12 `it_should_stop_quietly_if_ollama_is_unreachable_during_the_fallback`.
- [ ] 2.13 `it_should_keep_the_other_verdicts_of_a_call_when_one_prompt_changed_during_it`.
- [ ] 2.14 `it_should_log_each_call_and_each_verdict`: the call line has the categories, `fallback`, the input and output tokens and the duration; each verdict gets its own line.
- [ ] 2.15 `it_should_log_the_fallback`: the warning names the video, the reason and that each category is asked alone.
- [ ] 2.16 `it_should_return_an_auto_context_length_from_the_categories_with_a_prompt` (preferences handler): 9 categories give 16384 with `is_auto` true.
- [ ] 2.17 `it_should_return_4096_for_auto_with_few_categories`.
- [ ] 2.18 `it_should_override_the_categorization_context_length`: 8192 with `is_auto` false, whatever the number of categories.
- [ ] 2.19 `it_should_go_back_to_the_auto_context_length`: PATCH `null` gives the Auto value.
- [ ] 2.20 `it_should_keep_auto_when_saving_other_preferences`.
- [ ] 2.21 `it_should_ship_the_categorization_prompt_v2_batch`: `categorization_prompt.rs` includes `v2-batch.md`. Update `prompts/categorization/README.md` to mark v2-batch as shipped.
- [ ] 2.22 Web `aiSettingsForm.test.ts`, one cycle per case:
  - Auto on sends nothing.
  - Off with a new number sends the number.
  - Off without editing sends the shown value.
  - Back on sends `null`.
  - Off then on without saving sends nothing.

  Verify with `npm run test`.
- [ ] 2.23 Web `AiSettingsView.test.tsx`, one cycle per case:
  - "Auto context length" is on, with "Context length" read-only showing the value and the hint text from the spec.
  - Turning it off makes the field editable, holding the value.
  - Saving after turning it back on sends `null`.

  Verify with `npm run test`.

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 Ollama: `it_should_read_the_answer_and_tokens`, where `eval_count` gives `output_tokens`.
- [ ] 3.2 Ollama: rewrite `it_should_report_a_refusal_when_the_answer_takes_too_long` to use `chat.timeout` (100 ms); remove the `chat_timeout` field.
- [ ] 3.3 SQLite preferences: `it_should_read_the_old_default_context_length_as_auto`, where a stored 4096 reads as `None`.
- [ ] 3.4 SQLite preferences: `it_should_round_trip_an_overridden_context_length` (8192), with Auto stored as no value.
- [ ] 3.5 SQLite preferences: `it_should_read_an_invalid_categorization_context_length_as_auto`, replacing the old invalid-default expectation.

## 4. Eval and docs

- [ ] 4.1 `evals/categories/eval.py --batch`, mirroring the app's constants:
  - Auto `num_ctx`: 1,820 per category, rounded up to a multiple of 1,024, at least 4,096.
  - Per-call cap: 1,365 per category, at least 3,072, at most `num_ctx`. `--num-ctx` and `--max-output` still override.
  - The fallback: categories with an error from a call about several categories are asked alone, with rows `"fallback": true` and the failed call's stats in `"batch_call"`. Only final rows go into `verdicts.jsonl`.
  - `"fallback": true` in `run.json` and in the `--reuse` settings check.
  - A "Fallbacks" section in the report (videos, calls, output tokens, tokens spent on failed batch calls); the speed section counts both.

  Verify with `python3 -I evals/categories/eval.py --check`, then a `--limit 2` run against Ollama (or by hand if Ollama isn't available) that writes a report.
- [ ] 4.2 Run the batch eval on the cap-hit video (`IgJri10gl2M`, *1 in a Billion Checkmate?*), or the full set if time allows. Confirm the fallback answers its 9 categories, and add a `decisions.md` entry with the outcome, including whether batch-of-one with `v2-batch` matches `v2` on that video. Update the prompts README's eval column.
- [ ] 4.3 `evals/categories/README.md`: document the fallback and the Auto/cap defaults for `--batch`. Verify that the documented commands match `--help`.
- [ ] 4.4 `doc/AI.md` and `doc/CATEGORIES.md`: one call per video, the fallback, Auto context length (with how it scales), the scaled cap and timeout, and the `v2-batch` default. Verify that no doc still says "context 4096" or "once per category" for categorization.

## 5. Verification

- [ ] 5.1 `scripts/run-tests.sh --only web --only rust` passes (fmt, clippy, tests).
- [ ] 5.2 Manual (agent):
  1. Start `scripts/run-local.sh --port <free port> --fresh-videos`.
  2. Through the HTTP API, turn on AI and categorization with the Ollama URL, create 3 categories with AI prompts and add a short channel.
  3. With the Playwright MCP browser, open `/ai` → Categorization → Advanced. Check that "Auto context length" is on and the field is read-only at `6144` (Auto for 3 categories), with the hint shown.
  4. Turn it off, set `8192` and save. Check the PATCH body in the network requests.
  5. Turn it back on and save. Check that `null` is sent and the Auto value is shown again.
  6. If Ollama is reachable, let one video categorize and check the log shows one call line with 3 categories and 3 verdict lines.
  7. Stop the server, and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 5.3 Manual (human): on the MacBook Air or NAS with real Ollama and the 9 eval categories, categorize a few new videos. Confirm each takes one call (about 5 min) and that a long chess stream either answers or falls back without the timeout cutting it.
