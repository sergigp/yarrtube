# Tasks

The defaults used here (`claude-haiku-5-5`, effort `medium`) are provisional
until `add-anthropic-evals` reports its results. This change can be applied in
parallel. If the evals pick other defaults, update `AnthropicSettings::default`,
tests 2.5, 2.14 and 2.18, and the defaults in the specs, either through
`/opsx:update` or in a follow-up commit.

## 1. Walking skeleton

- [x] 1.1 Create every type and signature from design.md's Types & Signatures in the files it names, and wire them end to end. Bodies return trivial values; no `todo!()`. Done when `cargo build` succeeds and `scripts/run-tests.sh --lint --web --rust` passes with no new tests. The pieces:
  - **Domain.** `LlmConnection`, `Effort`, `AiProvider` and `AnthropicSettings`. The new `LlmChat`, `LlmModel` and `LlmServer` fields. `ReadLlmServerError::NotConfigured`.
  - **Preferences.** `Preferences::connection` and `apply(change, anthropic_available)`. The new `PreferencesChange` and `ResolvedPreferences` fields.
  - **Services.** `SummaryCall`, `CategorizationCall` and the job structs.
  - **Infrastructure.** Move the port and `FakeLlmServerRepository` to `llm_server_repository.rs`. `RoutingLlmServerRepository` delegates Ollama for real and returns an `LlmServerUnreachable` error for Anthropic. Stubs for `AnthropicLlmServerRepository`, `AnthropicConfig` and `AnthropicApiKey`.
  - **Config and composition root.** `InfrastructureSettings.anthropic` and `anthropic_available`. `serve.rs` reads `anthropic_config()`.
  - **HTTP.** `application/http/llm` replaces `http/ollama`: `/api/llm/ollama` works as before and `/api/llm/anthropic` answers 404. The preferences DTOs and `StoredPreferences` keys are added.
  - **Web.** The new `types.ts` fields, the `client.ts`/`queries.ts` functions with the moved Ollama path, and the fixture builder fields.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_list_ollama_models_at_the_llm_path`: `GET /api/llm/ollama` serves the Ollama check (move the existing http/ollama tests).
- [x] 2.2 `it_should_list_anthropic_models_in_the_listed_order`: `GET /api/llm/anthropic` returns the models in Anthropic's order.
- [x] 2.3 `it_should_answer_404_when_anthropic_isnt_configured`: no key gives a 404 with the configure message.
- [x] 2.4 `it_should_answer_502_when_anthropic_cant_be_reached`: an unreachable Anthropic gives a 502 `Could not reach Anthropic: `.
- [x] 2.5 `it_should_read_the_anthropic_defaults_and_availability`: the new preferences defaults and `anthropic_available`.
- [x] 2.6 `it_should_save_the_anthropic_provider_and_settings`: the provider and Anthropic settings are saved, and the Ollama settings are untouched.
- [x] 2.7 `it_should_reject_the_anthropic_provider_without_a_key`: 400 `Set YARRTUBE_ANTHROPIC_API_KEY to use Anthropic`.
- [x] 2.8 `it_should_reject_an_unknown_provider`: 400 for the `AiProvider` validation.
- [x] 2.9 `it_should_reject_an_unknown_effort`: 400 for the `Effort` validation.
- [x] 2.10 `it_should_enable_ai_on_anthropic_without_an_ollama_url`: "AI Needs An Ollama URL" applies only to Ollama.
- [x] 2.11 `it_should_ignore_a_sent_anthropic_available`: `anthropic_available` is read-only.
- [x] 2.12 `it_should_reset_failed_summaries_when_the_provider_changes`: `changes_summaries` compares the provider and the Anthropic summary settings.
- [x] 2.13 `it_should_reset_failed_verdicts_when_the_anthropic_categorization_effort_changes`: `changes_categorization` compares the provider and the Anthropic categorization settings.
- [x] 2.14 `it_should_summarize_with_the_anthropic_model_and_effort`: `SummaryCall::anthropic`, with the chat sent to `LlmConnection::Anthropic`.
- [x] 2.15 `it_should_budget_from_the_capped_anthropic_context`: the input budget comes from `min(window, 131072)`.
- [x] 2.16 `it_should_wait_when_the_anthropic_model_isnt_listed`: an unlisted model makes the summary wait 6 hours.
- [x] 2.17 `it_should_wait_when_anthropic_is_unavailable_during_a_call`: an unreachable-tagged chat error makes the summary wait 6 hours.
- [x] 2.18 `it_should_categorize_with_the_anthropic_model_and_effort`: `CategorizationCall::anthropic`, with the effort and schema sent.
- [x] 2.19 `it_should_fall_back_per_category_without_dropping_thinking_on_anthropic`: no retry without thinking on Anthropic.
- [x] 2.20 `it_should_count_an_own_call_limit_stop_as_a_failed_attempt_on_anthropic`: Retrying, with no `failed` verdict.
- [x] 2.21 `it_should_wait_when_anthropic_is_unavailable_for_categorization`: the categorization waits 6 hours and keeps its attempts.
- [ ] 2.22 Web: `it hides the provider choice without an Anthropic key`. The Provider section is unchanged without a key.
- [ ] 2.23 Web: `it saves only ai_provider when switching to Anthropic and shows its settings`. Covers the provider choice and saving it.
- [ ] 2.24 Web: `it lists Anthropic models and marks a saved one not listed as (not available)`. Covers the Anthropic model picker.
- [ ] 2.25 Web: `it shows Effort and Prompt only under Anthropic's Advanced and saves only the changed effort`. Covers the Effort field.
- [ ] 2.26 Web: `it shows "Connected to Anthropic, <n> models" after testing the connection`. Covers the Anthropic connection test.
- [ ] 2.27 Web: `it says the key isn't set when Anthropic is saved without one`. Covers the key-removed notice.

## 3. Infrastructure adapters (TDD)

`AnthropicLlmServerRepository`:

- [ ] 3.1 `it_should_send_the_api_key_and_version_headers`
- [ ] 3.2 `it_should_post_the_message_with_model_max_tokens_and_effort`
- [ ] 3.3 `it_should_send_the_schema_as_structured_output`
- [ ] 3.4 `it_should_send_no_temperature_or_thinking`
- [ ] 3.5 `it_should_read_the_text_blocks_and_usage`
- [ ] 3.6 `it_should_report_a_max_tokens_stop_as_stopped_at_the_limit`
- [ ] 3.7 `it_should_report_a_refusal_stop_as_refused`
- [ ] 3.8 `it_should_report_prompt_too_long_as_refused_as_too_long`
- [ ] 3.9 `it_should_report_another_bad_request_as_refused`
- [ ] 3.10 `it_should_report_a_low_credit_balance_as_unreachable`
- [ ] 3.11 `it_should_report_a_rejected_key_as_unreachable`
- [ ] 3.12 `it_should_report_an_unknown_model_as_unreachable`
- [ ] 3.13 `it_should_report_a_connect_error_as_unreachable`
- [ ] 3.14 `it_should_fail_on_an_overloaded_api_as_a_server_fault`
- [ ] 3.15 `it_should_list_models_across_pages`
- [ ] 3.16 `it_should_fail_the_check_on_an_answer_that_isnt_the_api`
- [ ] 3.17 `it_should_never_print_the_api_key`
- [ ] 3.18 `it_should_list_models_and_answer_on_the_real_api`: contract test, `#[ignore = "real Anthropic: run scripts/run-tests.sh --contract"]`. Make `--contract` run it only when `YARRTUBE_ANTHROPIC_API_KEY` is set, and skip it otherwise with a message saying why.

`RoutingLlmServerRepository`:

- [ ] 3.19 `it_should_route_an_ollama_connection_to_ollama`
- [ ] 3.20 `it_should_route_an_anthropic_connection_to_anthropic`
- [ ] 3.21 `it_should_report_anthropic_without_a_key_as_unreachable`

`SqlitePreferencesRepository`:

- [ ] 3.22 `it_should_round_trip_the_provider_and_anthropic_settings`
- [ ] 3.23 `it_should_read_preferences_without_provider_keys_as_their_defaults`

`serve.rs` config:

- [ ] 3.24 `it_should_have_no_anthropic_config_for_an_empty_key`
- [ ] 3.25 `it_should_default_the_anthropic_api_url`

## 4. Verification

- [ ] 4.1 Docs:
  - `doc/INSTALLATION.md`: add `YARRTUBE_ANTHROPIC_API_KEY` to the env table.
  - `doc/AI.md`: the provider choice, the Anthropic settings, costs, and the error mapping to waiting, server fault and rejected call.
  - `doc/ARCHITECTURE.md`: the routing repository and the two adapters.
  
  Check every env var and endpoint named in the docs against `serve.rs` and `http/mod.rs`.
- [ ] 4.2 Run `scripts/run-tests.sh --lint --web --rust`; every suite passes.
- [ ] 4.3 The e2e AI settings spec goes through the UI, so it follows the moved endpoint. Run `scripts/run-tests.sh --e2e` once to confirm nothing depends on `/api/ollama`.
- [ ] 4.4 Manual (agent):
  1. Start `scripts/run-local.sh --port <free port> --fresh-videos` without `YARRTUBE_ANTHROPIC_API_KEY`. Open `/ai` with the Playwright MCP browser and assert that no provider choice is shown.
  2. Restart with a real key. Assert that "Anthropic" is offered; select it, then "Test connection" shows `Connected to Anthropic, <n> models`.
  3. Save Anthropic with summaries on. Assert the `PATCH` body holds only the changed fields, and that "Advanced" shows Effort and Prompt only.
  4. Seed a channel with one downloaded, subtitled video through the HTTP API, let its summary run, and check the logs for the call to `claude-haiku-5-5` and the stored summary.
  5. Stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.5 Manual (human): read a few Haiku summaries and categorizations on your own library and judge them. Check the spend shown in the Anthropic Console against the per-video estimate in `doc/AI.md`.
