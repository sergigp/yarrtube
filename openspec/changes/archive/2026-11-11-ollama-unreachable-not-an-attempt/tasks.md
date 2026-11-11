# Tasks

No commits in this change unless the user asks for them (overrides the per-task commit in the apply guidance).

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures), wired with trivial bodies: `LlmServerUnreachable` (+ re-export), `UNAVAILABLE_SERVER_WAIT`, the new `Skip`/`Failure`/`Stop` variants with `from_chat_error` returning today's variant (`Server`), `check_server` keeping today's behaviour (categorizer still only checks with thinking on), `wait_for_server` unused or returning `Settled`, `send_error` returning `anyhow::Error::new(e)` untagged, fake `unreachable()` / `unreachable_error()` returning plain errors. No new behaviour, no new tests. Done when `cargo build` succeeds and `cargo test --locked` passes.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_wait_for_ollama_if_it_is_unreachable` (summarize task; replaces `it_should_count_ollama_down_as_a_failed_summary_attempt`) — an unreachable Ollama re-queues the summary 6 h later without touching the video.
- [x] 2.2 `it_should_wait_for_ollama_if_the_model_is_missing` (summarize task; replaces `it_should_count_a_missing_model_as_a_failed_summary_attempt`) — a missing model waits the same way.
- [x] 2.3 `it_should_keep_the_failed_attempts_of_a_retrying_summary_while_waiting_for_ollama` — a Retrying summary with 10 attempts keeps its counter and `last_failed_at`.
- [x] 2.4 `it_should_wait_for_ollama_if_it_goes_down_between_parts` — a connect failure on a later part's chat waits, storing no summary.
- [x] 2.5 `it_should_count_a_failed_server_check_as_a_failed_summary_attempt` — a server check that connected but failed (`failing()`) still spends an attempt (expected regression guard).
- [x] 2.6 `it_should_wait_for_ollama_if_it_is_unavailable_for_a_categorization` (categorize task; replaces `it_should_count_ollama_down_as_a_failed_categorization_attempt`) — unreachable, model missing, and connect failure on the first call each re-queue 6 h later with no verdicts and the categorization Pending.
- [x] 2.7 `it_should_not_fail_pending_categories_if_ollama_is_down_on_the_eleventh_attempt` (replaces `it_should_fail_pending_categories_if_ollama_is_down_on_the_eleventh_attempt`) — at 10 attempts, the same cases plus down during the fallback leave it Retrying with 10, write no `failed` verdict, and keep "Sicilian"'s stored verdict.
- [x] 2.8 `it_should_wait_for_ollama_if_the_model_is_missing_with_thinking_off` — drives the categorizer's always-on server check.
- [x] 2.9 `it_should_count_a_connection_dropped_during_the_fallback_as_a_failed_attempt` (rename of `it_should_stop_if_ollama_is_unreachable_during_the_fallback`) — a plain chat error still spends an attempt (expected regression guard).
- [x] 2.10 Update the two "Ollama down, or the model not installed" bullets in `doc/AI.md` to say the step waits 6 hours without spending an attempt; verify by reading them against the `video-retries` "Waiting For Ollama" requirement.

## 3. Infrastructure adapters (TDD)

`ollama_llm_server_repository.rs`:

- [x] 3.1 `it_should_report_an_unreachable_server_on_chat` (tightens `it_should_fail_if_the_server_is_unreachable`) — a closed port's chat error is `LlmServerUnreachable`.
- [x] 3.2 `it_should_report_an_unreachable_server_on_find` — a closed port's `find` error is `LlmServerUnreachable`.
- [x] 3.3 `it_should_not_report_a_dropped_connection_as_unreachable` (tightens `it_should_fail_if_the_server_drops_the_connection`) — expected regression guard.
- [x] 3.4 `it_should_not_report_a_slow_server_check_as_unreachable` — a listener that accepts and never answers fails `find` untagged (expected regression guard).

## 4. Verification

- [x] 4.1 Run `scripts/run-tests.sh --only rust` (fmt, clippy, tests); all pass. Backend-only change, so the web suite is not run.
- [x] 4.2 `openspec validate ollama-unreachable-not-an-attempt --strict` passes.
