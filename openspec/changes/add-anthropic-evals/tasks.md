# Tasks

The evals are Python scripts with no test suite. Each task is checked with a
real run limited by `--case` / `--limit`, or with `--check`, rather than a
red-green cycle.

## 1. Walking skeleton

- [x] 1.1 Add `--provider ollama|anthropic` (default `ollama`) and `--effort low|medium|high` (default `medium`) to both evals, stored in a new run's `meta` and kept by a resumed run. Add an `anthropic_chat(model, system, user, fmt, effort, max_out, timeout)` stub in `evals/summaries/eval.py` that both evals route to when the provider is `anthropic`, and fail early with a clear message when `YARRTUBE_ANTHROPIC_API_KEY` is unset. Done when `python3 -I evals/summaries/eval.py --check` and `python3 -I evals/categories/eval.py --check` pass and an Ollama run behaves exactly as before (same `meta` apart from the new keys).

## 2. Behaviour

- [ ] 2.1 Summaries with `--provider anthropic`: cut parts with a 131072-token context, send `max_tokens` 16000, read the text blocks as the answer, and record `in`/`out` from `usage`. Verify with `--provider anthropic --model claude-haiku-5-5 --case browser-tabs`: a summary row is written and the call stats hold tokens.
- [ ] 2.2 Categories with `--provider anthropic`: send the answer's JSON schema as `output_config.format` and the eval's output cap as `max_tokens`. Treat `stop_reason: max_tokens` as "hit the output limit" and `refusal` as an error row. Verify with `--provider anthropic --model claude-haiku-5-5 --batch --limit 2`: verdict rows are written.
- [ ] 2.3 Show provider and effort in both reports, and print the run's input and output token totals with an estimated cost from a price table in the script. Verify the report header and the printed totals of the runs from 2.1 and 2.2.

## 3. Infrastructure adapters

- [ ] 3.1 Implement `anthropic_chat` as `POST https://api.anthropic.com/v1/messages` with headers `x-api-key`, `anthropic-version` and `content-type`. Send no `temperature` and no `thinking`. Verify by hand with the 2.1 run, and by inspecting one logged request body.
- [ ] 3.2 Error handling in `anthropic_chat`:
  - Stop the run with a resumable message on: a connect error, 401/403, 404 (unknown model), or a 400 whose message says the credit balance is too low.
  - Return an error row on: any other 400, including "prompt is too long".
  - Raise on 429 / 5xx / 529, as the Ollama path does for non-400s.
  
  Verify a 401 with `YARRTUBE_ANTHROPIC_API_KEY=bogus`: the run stops with the message and `--run` resumes it.

## 4. Verification

- [ ] 4.1 Run the summaries eval on `claude-haiku-5-5` at effort `low` and `medium`, judged by the external Sonnet judge as for the baseline. Compare must-haves, major errors and coverage against the latest `qwen3:4b` run, and record the results, the effort chosen and the token cost in `evals/summaries/decisions.md`.
- [ ] 4.2 Run the categories eval on `claude-haiku-5-5` (`--batch`, the shipped categorization prompt) at effort `low` and `medium`. Compare precision and recall against the latest `qwen3:4b` run, and record the results and the effort chosen in `evals/categories/decisions.md`.
- [ ] 4.3 Document `--provider`, `--effort` and `YARRTUBE_ANTHROPIC_API_KEY` in `evals/summaries/README.md` and `evals/categories/README.md`, and check that the documented commands run as written.
- [ ] 4.4 If the results change a default (model or effort) assumed by `add-anthropic-provider`, update that change's artifacts with `/opsx:update` before it is applied.
