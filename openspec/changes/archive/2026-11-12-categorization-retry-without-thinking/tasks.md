# Tasks

Note: the user asked for no commits or pushes unless they ask; this overrides the apply guidance's per-task commit.

## 1. Walking skeleton

- [x] 1.1 Create every type and signature from design.md, wired end to end, bodies trivial: `LlmReply.thinking` + `thinking_tail()` (returns `""`), `THINKING_TAIL_CHARS`; every `LlmReply { .. }` literal (adapter, fake replies, test helpers) gets `thinking: String::new()`; `StepState::give_up` / `Video::give_up_categorization` (return `self`); in `VideoCategorizer` the `Thinking`, `Rejection`, `RejectionKind` types, `retry_without_thinking` (returns `rejected` unchanged), `settle` (calls `mark_categorized`), `log_limit_stop` / `log_given_up` (empty), `ask` / `fall_back` / `give_up` / `log_call` taking the new parameters with every rejection `Retryable`; `AiStepCounts` replacing `StepCounts` for summaries and categorizations with `queued` filled from today's pending video count; heartbeat fields renamed to `summaries_queued` / `categorizations_queued`. Done when `cargo build` succeeds and all existing tests pass.

## 2. Behaviour (TDD)

- [x] 2.1 Switch `it_should_ask_each_category_alone_when_the_call_hits_the_output_limit`, `it_should_not_fall_back_for_a_single_category` and `it_should_log_the_fallback` to a model that can't think (they describe today's behaviour, kept for non-thinking calls); they must still pass.
- [x] 2.2 `it_should_ask_again_without_thinking_if_a_single_category_call_thought_past_the_limit` — a thinking call about one category that stops at the limit is asked once more without thinking; its verdict stored, categorization Done.
- [x] 2.3 `it_should_ask_every_category_again_without_thinking_if_the_call_thought_past_the_limit` — the call about all pending categories is retried as one call without thinking, before any fallback.
- [x] 2.4 `it_should_ask_alone_without_thinking_what_the_call_without_thinking_left_unanswered` — fallback after the thinking-off call stays without thinking.
- [x] 2.5 `it_should_ask_again_without_thinking_if_a_fallback_call_thought_past_the_limit` — a fallback single call with thinking that hits the limit gets the thinking-off retry.
- [x] 2.6 `it_should_give_up_at_once_if_the_call_without_thinking_is_rejected` — `failed` verdict, categorization Failed with 1 failed attempt, nothing scheduled.
- [x] 2.7 `it_should_give_up_one_category_and_retry_another_in_the_same_run` — given-up category gets `failed`, the retryable one keeps today's failed attempt and re-queue.
- [x] 2.8 `it_should_mark_the_categorization_failed_if_a_category_was_given_up_before` — a run that answers everything left but finds a `failed` verdict ends Failed, not Done.
- [x] 2.9 `it_should_not_give_up_a_category_if_the_settings_changed_during_its_call_without_thinking` — no `failed` verdict; categorization Pending with no failed attempts.
- [x] 2.10 `it_should_keep_todays_retry_if_the_call_hit_the_limit_without_thinking` (renamed in 4.4) — thinking off in settings: one call, Retrying, re-queued.
- [x] 2.11 `it_should_log_whether_each_call_thought_and_retried_without_thinking` — `thinking` and `retry_without_thinking` on each `asked the model` line.
- [x] 2.12 `it_should_log_how_the_output_was_spent_when_a_call_stops_at_the_limit` — warn line with `thinking_chars`, `answer_chars`, `thinking_tail` (drives `thinking_tail()`).
- [x] 2.13 `it_should_log_a_category_given_up_at_once` — error line with the video, category and reason.
- [x] 2.13b `it_should_mark_the_categorization_failed_without_asking_if_only_a_given_up_category_is_left` — audit fix: with nothing pending, a video holding a `failed` verdict ends Failed, not Done.
- [x] 2.14 `it_should_log_how_the_output_was_spent_when_a_summary_call_stops_at_the_limit` (summarize task tests) — the same warn line from `VideoSummarizer::ask`.
- [x] 2.15 `it_should_count_ai_tasks_due_not_pending_videos` (daemon health searcher) — `queued` from pending `summarize_video` / `categorize_video` tasks due now (deferred ones excluded, audit fix); update `it_should_report_queue_and_library_counts` to `AiStepCounts`.

## 3. Infrastructure adapters (TDD)

### Ollama

- [x] 3.1 `it_should_read_the_thinking` — `message.thinking` read into `LlmReply.thinking`, empty when absent.

## 4. Verification

- [x] 4.1 Update `doc/AI.md`: categorization sequence (a)/(b), retry without thinking, give up at once and Failed-with-`failed`-verdict, `retry_without_thinking` and the limit-stop warn line (categorization and summaries), heartbeat `*_queued`; replace the stale `unavailable` verdict naming with `failed`; note the summary output-limit follow-up.
- [x] 4.2 `scripts/run-tests.sh --only rust` (fmt, clippy, tests) passes. No web code changes, so `--only web` is not needed.
- [x] 4.3 Audit follow-up (refactor, no behaviour change): `log_call` takes `thinking: Thinking` and the resolved `thinks: bool` instead of two bools, deriving `retry_without_thinking` from `Thinking::Dropped`, as in design.md. Existing log tests must pass unchanged.
- [x] 4.4 Audit follow-up: rename `it_should_keep_todays_retry_if_the_call_hit_the_limit_without_thinking` to `it_should_retry_later_if_a_call_without_thinking_hit_the_limit`.
- [x] 4.5 Re-run `scripts/run-tests.sh --only rust` after 4.3–4.4.
