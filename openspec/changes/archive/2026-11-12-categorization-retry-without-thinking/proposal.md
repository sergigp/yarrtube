# Proposal

## Why

On the NAS, 3 of 27 categorization calls in two hours (2026-11-11) stopped at the 3,072-token output cap with thinking on, after ~5.5 minutes each, before writing a ~100-token answer. Temperature is 0 with no seed, so every retry is the identical call with the identical result: today the category is rejected and the per-video retry policy repeats that call up to 11 times over ~7 days, ~5.5 minutes of the single LLM lane each time, before giving up anyway. Separately, the heartbeat's `summaries_pending=1233` / `categorizations_pending=1330` count every video whose step is Pending, including sources and libraries with the step off, so they say nothing about the AI backlog (~20 tasks actually queued).

## What Changes

- **Retry without thinking.** When a categorization call stops at the output limit with thinking on, the same categories are asked once more, in the same run, with thinking off. Its answers are stored like any other verdict.
- **Exact call sequences** (N = pending categories, thinking model, thinking on):
  - (a) N ≥ 2, the call about all N stops at the limit: 1) all N, thinking → limit; 2) all N, no thinking; 3) each category that call left without a usable answer, alone, no thinking. At most 2 + N calls. The categories are not also asked alone *with* thinking: the thinking-off call about all of them is the cheap path (~1 call instead of N runaway-prone ones).
  - (b) A call about one category stops at the limit (N = 1, or a fallback call after the all-N call was refused or partly unusable): that category alone, no thinking. One extra call.
  - Other rejections (refused, past the timeout, unusable answer) with thinking on keep today's path: all N → each rejected alone (with thinking) → (b) if one of those hits the limit.
- **Later attempts: give up at once.** A category rejected by a call made without thinking *because* a thinking call hit the limit (step 3 of (a), or the extra call of (b)) gets a `failed` verdict in that run instead of spending the remaining attempts. Justification: at temperature 0 a later attempt replays the same calls (thinking → limit, then the same thinking-off call) with the same result, so the 10 remaining attempts would cost ~55 min+ of the LLM lane for nothing. `failed` already means "the AI gave up on the pair under the current settings", and is already undone by the existing paths (Retry, changing model / context / temperature / thinking / prompt, a new AI prompt for the category). Persisting "skip thinking for this pair" so later attempts go straight to thinking off was rejected: it needs schema and the thinking-off call would still repeat identically.
  - Categorization status after such a run: Retrying (today's failed attempt) if any other category is still rejected; otherwise **Failed** (not Done), so a video holding a `failed` verdict always shows as failed and the Retry button applies. A later run that leaves nothing rejected but finds the video still holds a `failed` verdict also ends Failed, not Done.
- **Unchanged:** a limit stop with thinking off, or with a model that can't think, keeps today's behaviour (fallback, failed attempt, 11 attempts).
- **Logging.**
  - Every "asked the model" line gains `retry_without_thinking` (true for the thinking-off call made after a limit stop).
  - On any limit stop (categorization and summaries): a warn line with `thinking_chars` and `answer_chars` (Ollama's `eval_count` doesn't split thinking from answer, so characters, not tokens) and `thinking_tail`, the last ~300 characters of the thinking. Warn, not debug: it's rare (3/27) and it's the evidence for whether the model was circling, which production logs at info must keep.
  - A category given up at once is logged at error level with its reason.
- **Heartbeat.** `summaries_pending` / `categorizations_pending` are replaced by `summaries_queued` / `categorizations_queued`: pending `summarize_video` / `categorize_video` tasks due to run (a retry or a wait for Ollama queued for later isn't counted, so they don't overlap `*_retrying`). Filtering videos by "wants the step" would need an owner lookup and a verdict read per video every 5 minutes (thousands of queries); the queued task count is what is actually waiting for the AI and comes from the task list the heartbeat already reads. `*_retrying` / `*_failed` stay (a video only reaches them by being attempted). **BREAKING** for anything parsing those two log fields.
- **Docs.** `doc/AI.md`: the sequence, give-up-at-once, the new log fields; also fixes its stale `unavailable` verdict naming (code and spec say `failed`).

### Findings, no change

- **Summaries (item 3).** The two 5 Nov dead letters ("hit its 3288-token output limit") predate #159 (merged 5 Nov 14:27), which replaced the fixed 3,288-token output budget by "whatever the context leaves". With inputs of 7,997 / 9,470 tokens in 12,288, that leaves 4,291 / 2,818 tokens for thinking plus a multi-hundred-token summary; the evals average ~5 min of thinking per summary (thousands of tokens). So the likely cause is a normal thinking pass plus a long answer not fitting the room the input budget leaves, not runaway thinking on a tiny answer. Retrying without thinking might help but is not clearly the same fix (free-text answer, input-budget sizing is the other lever). **Follow-up**, not in this change; this change adds the limit-stop log line to summaries too, which gives the thinking-vs-answer evidence to decide it.
- **Verdict provenance (item 4).** A stored verdict records only `model`, `reason`, `decided_by`, `decided_at`: not thinking or other settings. So a verdict answered without thinking can't be told apart from one answered with it; per the brief, no schema is added. The `retry_without_thinking=true` log line is the only record.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `video-categorization`: Rejected Category Calls (retry without thinking, give up at once), Categorization Status (Failed when a `failed` verdict remains), Categorization Logged (new fields, limit-stop line).
- `video-retries`: Giving Up After A Week (categorization may give up a category before the 11th attempt).
- `video-summaries`: Summary Generation Logged (limit-stop line).
- `daemon`: Heartbeat Logging (queued AI tasks instead of pending videos).

## Impact

- Code: `src/domain/services/video_categorizer.rs` (sequence, give-up, logs), `src/domain/services/video_summarizer.rs` (limit-stop log), `src/domain/llm_server/llm_reply.rs` (+ `thinking`), `src/infrastructure/repositories/ollama_llm_server_repository.rs` (read `message.thinking`), `src/domain/video/` (Failed-now transition), `src/domain/daemon/daemon_health.rs`, `src/domain/services/daemon_health_searcher.rs`, `src/serve.rs` (heartbeat fields).
- Tests: categorize task tests (`src/application/tasks/categorize_video_task/tests.rs`), three existing limit-stop tests change expectations; Ollama adapter; daemon health searcher.
- Docs: `doc/AI.md`. No schema, API or UI change.
- Out of scope: the cap formula, thinking on by default, prompts, categories, the retry schedule.
