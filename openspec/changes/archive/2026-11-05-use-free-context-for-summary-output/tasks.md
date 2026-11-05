## 1. Walking skeleton

- [x] 1.1 Nothing to scaffold: no new file, type or port (design.md, Types & Signatures). Done when `cargo build` succeeds and existing tests pass before the first cycle.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_send_the_saved_model_settings`: expect `max_output_tokens: 8192`, and `12288` in the `one_call_chat` fixture; send the context length from `VideoSummarizer::ask` and remove `ContextLength::output_budget_tokens`. Verify: `cargo test summarize_video` passes.
- [x] 2.2 `it_should_fail_the_task_when_the_answer_stopped_at_the_limit`: expect `the model ran out of context (12288 tokens: 100 of input, 12188 left to think and answer)`, using a saturating subtraction for the room left. Verify: `cargo test summarize_video` passes.

## 3. Infrastructure adapters (TDD)

- [x] 3.1 No adapter changes (design.md, Test Plan). Verify: `git diff --stat` shows no change under `src/infrastructure/`.

## 4. Verification

- [x] 4.1 Update `doc/AI.md` (context limit wording) and `evals/summaries/decisions.md` (qwen3 output up to 3,551 tokens, `num_predict` gotcha). Verify: `grep -rn 'output cap' doc evals/summaries/decisions.md src` finds nothing.
- [x] 4.2 Run `scripts/run-tests.sh --only rust` and verify fmt, clippy and tests pass (no web files change).
- [ ] 4.3 Manual (human): after deploying, check the next logs report for fewer context-limit failures, and read their `left to think and answer` values to decide whether the input budget needs retuning.
