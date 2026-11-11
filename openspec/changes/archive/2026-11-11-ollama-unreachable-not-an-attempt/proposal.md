# Proposal

## Why

An unreachable Ollama server is recorded as a failed attempt on every video waiting for a summary or a categorization. After about 7 days 7 hours of Ollama being down (11 attempts: 10m, 1h, 6h, then 24h ×7) every waiting video is Failed, categorizations write `failed` verdicts, and nothing retries them when Ollama comes back: the user has to retry each video by hand or change a setting to bulk-reset. Ollama being down says nothing about the video, so it shouldn't spend the video's attempts.

## What Changes

- Ollama being **unavailable** stops a summary or categorization run without recording a failed attempt: the step's status and attempt counter stay as they were (a Retrying step keeps its counter and its last failure time), no `failed` verdict is written, and the step's task is re-queued **6 hours** later.
- Unavailable means:
  - **Can't connect**: connection refused, DNS failure or the 5-second connect timeout, whether on the server check before the run or on any chat call during it (Ollama going down mid-run; verdicts already stored in that run are kept).
  - **Saved model not installed**, as seen on the server check before the run. Included because, like Ollama down, it is the same for every video and is fixed outside Yarrtube (`ollama pull`), which changes no setting, so nothing would reset the Failed videos. Burning attempts on it reproduces the same week-long doom.
- The categorization now always checks the server (version, models) before asking, not only when thinking is on, so a missing model is seen the same way for both steps.
- Still failed attempts (unchanged): stopped at the output limit, unusable answer, refusal (400), a call past its timeout after the connection was accepted, any other server error (5xx, a 404 on a chat call, a non-Ollama answer, a server check that connects but fails or times out reading), an unreadable transcript.
- The fixed 6 h wait is not a backoff and has no limit: a step waits for Ollama indefinitely, at one cheap check per waiting step every 6 hours.
- **BREAKING (behaviour)**: the `video-retries` rule "Ollama being unreachable … or the saved model not being installed" counts as a failed attempt is reversed, and the "Summary gives up while Ollama is down" scenario now says the summary does not give up.
- `doc/AI.md`'s two "Ollama down, or the model not installed" lines are updated to say so.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `video-retries`: "What Counts As A Failed Attempt" no longer counts Ollama unavailable; new "Waiting For Ollama" requirement (re-queue 6 h later, counter untouched); "Giving Up After A Week"'s Ollama-down scenario no longer gives up.
- `video-summaries`: "Ollama Unavailable" waits instead of failing an attempt, including Ollama becoming unreachable during generation; "Failed Summaries Follow The Retry Policy" drops "Ollama unreachable" from server faults.
- `video-categorization`: "Ollama Unavailable For Categorization" waits instead of failing an attempt and never writes `failed` verdicts; covers can't-connect on any call of the run, including the fallback.

## Impact

- `src/domain/llm_server/errors.rs`: new `LlmServerUnreachable` marker error.
- `src/infrastructure/repositories/ollama_llm_server_repository.rs`: tags connect errors of `find` and `chat` with it; the fake gets an `unreachable()` server.
- `src/domain/video/retry_attempts.rs`: the 6 h wait constant.
- `src/domain/services/video_summarizer.rs`, `video_categorizer.rs`: classify unavailable and return a re-queue without touching the video.
- Tests in `src/application/tasks/summarize_video_task.rs`, `categorize_video_task.rs`; adapter tests.
- No schema, API or UI change. Videos already Failed because of a past Ollama outage are not reset by this change (retry them, or change a setting).
