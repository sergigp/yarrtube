# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every type and signature from design.md (## Types & Signatures), wired end-to-end with trivial bodies:
  - `caption_lines` returning `vec![strip_tags(line)]` minus empties, used by both `from_srt`s.
  - `SummaryMessages::chars`.
  - `SummaryInput::new` taking `prompt` and `chars_per_token` (callers pass `language.chars_per_token()`). The prompt is stored, and `whole`/`part`/`reduce`/`reduce_group` lose their prompt param. `over_budget` returns `false`; `budget_tokens` is stored.
  - `Failure::OverBudget(Overrun)` and `Overrun`.
  - `transcript` / `summary_input` / `write_summary_cut_smaller_once`, with the last one calling `write_summary` once.
  - `ask` taking `input`.
  - Update existing test helpers (`whole_messages`, `part_messages`, …) to the new signatures only.

  Done when `cargo build` succeeds and `cargo test --locked` passes unchanged.
- [x] 1.2 Skeleton for the refusal-as-overrun revision: `LlmReply.exceeded_context` (false everywhere it's built today) and `LlmReply::refused_as_too_long`; `Overrun.input_tokens: Option<u32>` (`Some` where it's set today), logged as the count; `VideoSummarizer::exceeded_context` returning `Failure::Rejected` for now, not yet called. Done when `cargo build` succeeds and `cargo test --locked` passes unchanged.

## 2. Behaviour (TDD)

- [x] 2.1 `fetch_subtitles_task::it_should_store_hard_spaces_as_spaces_in_the_transcript`: `\h` becomes a space, spaces are collapsed and the line trimmed in the stored transcript. Verify: the test passes, along with `cargo test --locked`.
- [x] 2.2 `fetch_subtitles_task::it_should_store_a_forced_line_break_as_a_line_break`: `\N` splits a caption line into two stored lines. Verify: the test passes, along with `cargo test --locked`.
- [x] 2.3 `summarize_video_task::it_should_send_the_transcript_without_hard_spaces`: the timed lines read from the `.srt` reach the user message clean. Verify: the test passes, along with `cargo test --locked`.
- [x] 2.4 `summarize_video_task::it_should_cut_in_parts_when_the_prompt_pushes_the_call_over_the_budget`: the whole call is budgeted on `whole().chars()`, not the transcript. Adjust existing boundary tests only where the prompt overhead now shifts their cut, keeping what each asserts. Verify: the test passes, along with `cargo test --locked`.
- [x] 2.5 `summarize_video_task::it_should_keep_every_part_call_within_the_budget`: the part transcript limit is `max_chars` minus the map messages' overhead. Verify: the test passes, along with `cargo test --locked`.
- [x] 2.6 `summarize_video_task::it_should_reduce_in_groups_when_the_reduce_prompt_pushes_the_answers_over_the_budget`: `reduce_fits`/`reduce_groups` are checked on the rendered reduce messages. Verify: the test passes, along with `cargo test --locked`.
- [x] 2.7 `summarize_video_task::it_should_cut_smaller_when_a_part_runs_over_the_budget`: `over_budget` is real, `ask` returns `OverBudget`, and `write_summary_cut_smaller_once` rebuilds the input at the measured ratio and starts over. Verify: the test passes, along with `cargo test --locked`.
- [x] 2.8 `summarize_video_task::it_should_cut_in_parts_when_the_whole_transcript_runs_over_the_budget`: an overrun on the whole call leads to parts + reduce on the second pass. Verify: the test passes, along with `cargo test --locked`.
- [x] 2.9 `summarize_video_task::it_should_fail_the_summary_if_the_input_runs_over_the_budget_again`: a second-pass `OverBudget` becomes `Rejected` with the "ran out of context" reason. Verify: the test passes, along with `cargo test --locked`.
- [x] 2.10 `summarize_video_task::it_should_log_the_cut_smaller_with_the_measured_ratio`: the info log names the call, input tokens, budget and chars/token. Verify: the test passes, along with `cargo test --locked`.
- [x] 2.12 `summarize_video_task::it_should_schedule_the_next_attempt_if_the_input_is_exactly_the_budget` (added by the audit): exactly the budget is not over it.
- [x] 2.13 `summarize_video_task::it_should_reduce_more_than_two_answers_in_a_group_when_they_fit` (added by the audit): a group takes as many answers as fit.
- [x] 2.14 `summarize_video_task::it_should_cut_smaller_when_the_prompt_exceeds_the_context`: `ask` turns a refusal with `exceeded_context` into `OverBudget` through `exceeded_context`, at `messages.chars() / context_length`. Verify: the test passes, along with `cargo test --locked`.
- [x] 2.15 `summarize_video_task::it_should_fail_the_summary_if_the_prompt_exceeds_the_context_again`: a second refusal as too long fails the attempt with Ollama's reason. Verify: the test passes, along with `cargo test --locked`.
- [x] 2.16 `summarize_video_task::it_should_log_the_cut_smaller_after_a_prompt_over_the_context`: `log_cut_smaller` shows `input_tokens=over the context length` when there's no count. Verify: the test passes, along with `cargo test --locked`.
- [x] 2.17 Update `doc/AI.md` "How a summary is generated": a prompt Ollama refuses as over the context length is cut smaller once too. Verify: the doc matches the spec.
- [x] 2.11 Update `doc/AI.md` "How a summary is generated" with the escape codes cleaned, the budget on whole messages, and the single re-cut at the measured ratio (logged). Add a dated entry to `evals/summaries/decisions.md` with the measured ratios: 11,932 reproduced, `\h` costing about 2.2k tokens, chess English at 3.75, Spanish at 3.21–3.44. Verify: both docs describe the behaviour in the specs.

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 Ollama adapter: `it_should_report_a_refusal_as_too_long_when_the_prompt_exceeds_the_context` (retarget `it_should_report_a_refusal_on_a_bad_request` to another 400 error first). Verify: the test passes, along with `cargo test --locked`.
- [ ] 3.2 Ollama adapter: `it_should_report_a_refusal_as_too_long_when_the_input_length_exceeds_the_context`. Verify: the test passes, along with `cargo test --locked`.

## 4. Verification

- [x] 4.1 `scripts/run-tests.sh --only rust` passes: fmt, clippy, tests. The change has no web side.
- [ ] 4.4 After the refusal-as-overrun revision: `scripts/run-tests.sh --only rust` passes.
- [ ] 4.2 Manual (agent): run `scripts/run-local.sh --port <free port> --fresh-videos` against the local Ollama with `qwen3:4b`, seed a source with summaries on and the chess `.srt` (`/private/tmp/claude-501/-Users-sergigp--herdr-worktrees-yarrtube-summary-input-overrun/6bd9f37f-6005-45a5-9621-85635ee5d27d/scratchpad/srt/chess.en.srt`) as its subtitles, and let the summary run. Expected: in the logs, every call's `input_tokens` is ≤ 9,000, or one "cutting the transcript smaller" line then all calls ≤ 9,000, and the summary is stored. Then stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.3 Manual (human): after the `:edge` image is on the NAS, confirm chess video `eef9c810-4b8e-46e9-9591-0a68d82da349` (Retrying, next attempt 2026-11-13 05:53 UTC) ends Done, and that its log lines show the input within budget.

## Workflow follow-up

- Run `/audit-change`, then archive the change once reviewed.
