# Design

## Files

- `src/domain/llm_server/errors.rs` — new `LlmServerUnreachable` marker error, next to `ReadLlmServerError` (same pattern as `StaleVideo`: adapter raises it inside `anyhow`, domain checks `is::<_>()`).
- `src/domain/llm_server/mod.rs` — re-export it.
- `src/domain/video/retry_attempts.rs` — `UNAVAILABLE_SERVER_WAIT` (6 h), beside the backoff it sidesteps.
- `src/domain/services/video_summarizer.rs` — classify unavailable (server check or chat) and wait instead of `record_failure`.
- `src/domain/services/video_categorizer.rs` — always check the server; classify unavailable (check or any chat) and wait instead of `fail_everything`.
- `src/infrastructure/repositories/ollama_llm_server_repository.rs` — tag connect errors of `find` and `chat` with the marker; fake gains `unreachable()` and `unreachable_error()`.
- `src/application/tasks/summarize_video_task.rs`, `categorize_video_task.rs` — acceptance tests.
- `doc/AI.md` — the two "Ollama down, or the model not installed" bullets.

No entity transition: waiting leaves the `Video` untouched, so no `Video`/`StepState` method is added.

## Types & Signatures

```rust
// domain/llm_server/errors.rs
/// Ollama couldn't be connected to (refused, unknown host, connect timeout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmServerUnreachable;
impl fmt::Display for LlmServerUnreachable { /* "can't connect to Ollama" */ }
impl std::error::Error for LlmServerUnreachable {}

// domain/video/retry_attempts.rs
/// How long a summary or categorization waits for an unavailable Ollama,
/// without spending an attempt.
pub const UNAVAILABLE_SERVER_WAIT: Duration = Duration::hours(6);

// domain/services/video_summarizer.rs
enum Skip {
    NotWanted(&'static str),
    Blocked(anyhow::Error),
    /// Can't connect, or the model isn't installed: wait, no attempt spent.
    Unavailable(anyhow::Error),
    /// The server check connected but failed: a failed attempt.
    Failed(anyhow::Error),
}
enum Failure {
    /// Can't connect during a call: wait, no attempt spent.
    Unavailable(anyhow::Error),
    Server(anyhow::Error),
    Rejected(anyhow::Error),
}
impl Failure { fn from_chat_error(error: anyhow::Error) -> Self; }
impl VideoSummarizer {
    fn check_server(&self, url: &OllamaUrl, settings: &SummarySettings) -> Result<LlmModel, Skip>;
    fn wait_for_server(&self, video: &Video, reason: anyhow::Error) -> VideoStepOutcome;
}

// domain/services/video_categorizer.rs
enum Skip {
    NotWanted(&'static str),
    Blocked(anyhow::Error),
    /// Can't connect, or the model isn't installed: wait, no attempt spent.
    Unavailable(anyhow::Error),
    /// The server check connected but failed: a failed attempt of the job.
    Failed(anyhow::Error, Box<CategorizationJob>),
}
enum Stop {
    /// Can't connect during a call: wait, no attempt spent.
    Unavailable(anyhow::Error),
    Server(anyhow::Error),
    Failed(anyhow::Error),
}
impl Stop { fn from_chat_error(error: anyhow::Error) -> Self; }
impl VideoCategorizer {
    /// Replaces `thinking`: always runs.
    fn check_server(&self, job: &CategorizationJob) -> Result<bool, Skip>;
    fn wait_for_server(&self, video: &Video, reason: anyhow::Error) -> VideoStepOutcome;
}

// infrastructure/repositories/ollama_llm_server_repository.rs
impl OllamaLlmServerRepository {
    /// `error`, tagged `LlmServerUnreachable` when it is a connect error.
    fn send_error(error: reqwest::Error) -> anyhow::Error;
}
#[cfg(test)] impl FakeLlmServerRepository {
    /// A server nothing can connect to: `find` fails with `unreachable_error()`.
    pub fn unreachable() -> Self;
}
#[cfg(test)] pub fn unreachable_error() -> anyhow::Error; // "connection refused" + LlmServerUnreachable
```

`FakeLlmServerRepository::failing()` keeps failing with a plain error (a server check that connected but failed).

## Call Stack

Summary, Ollama unavailable:
```
SummarizeVideoTask::handle(task)
  VideoSummarizer::summarize(&video)
    prepare(&video)
      check_server(&url, &settings)
        llm_server_repository.find(&url)        Err(e) if e.is::<LlmServerUnreachable>() -> Skip::Unavailable(e)
                                                 Err(e)                                    -> Skip::Failed(e)
        server.model(model)                     None                                      -> Skip::Unavailable(anyhow!("{model} isn't installed on Ollama"))
    Skip::Unavailable(reason) -> wait_for_server(&video, reason)
                                   -> VideoStepOutcome::RetryAt { at: clock.now() + UNAVAILABLE_SERVER_WAIT, reason: cause_chain(&reason) }
    Skip::Failed(reason)      -> record_failure(&video, cause_chain(&reason))     // unchanged path
    generate(&video, &job) -> try_generate -> write_summary -> ask
      llm_server_repository.chat(&url, &chat).map_err(Failure::from_chat_error)
    Failure::Unavailable(reason) -> wait_for_server(&video, reason)
    Failure::Server | Rejected  -> record_failure(..)                              // unchanged
  TaskOutcome::from(outcome)    // RetryAt -> RunAgainAt { run_at, reason }
```

Categorization, Ollama unavailable:
```
CategorizeVideoTask::handle(task)
  VideoCategorizer::categorize(&video)
    prepare(&video)
      check_server(&job)      find Err(unreachable) | model None -> Skip::Unavailable(e)
                              find Err(other)                     -> Skip::Failed(e, Box::new(job))
                              Ok(model) -> thinking = settings.thinking && model.thinking
    Skip::Unavailable(reason) -> wait_for_server(&video, reason)
    Skip::Failed(reason, job) -> fail_everything(&video, &job, reason)           // unchanged path
    ask_each(&video, &job) -> rejected_categories -> ask / fall_back -> ask
      llm_server_repository.chat(..).map_err(Stop::from_chat_error)
    Stop::Unavailable(reason) -> wait_for_server(&video, reason)                 // verdicts already stored stay
    Stop::Server(reason)      -> fail_everything(..)                              // unchanged
```

Adapter:
```
find -> get_json -> read_json: request.send().map_err(Self::send_error)?
chat: Err(e) if e.is_timeout() && !e.is_connect() -> refused reply (unchanged)
      Err(e) -> Err(Self::send_error(e))
send_error(e): e.is_connect() -> anyhow::Error::new(e).context(LlmServerUnreachable)
               else           -> anyhow::Error::new(e)
```

`LlmServerSearcher` (Test connection) is unaffected: it maps every `find` error to 502 as before.

## Test Plan

1. Behaviour tests (tasks, real SQLite, `FakeLlmServerRepository`, `FixedClock`)

`summarize_video_task.rs`:
1. `it_should_wait_for_ollama_if_it_is_unreachable` (replaces `it_should_count_ollama_down_as_a_failed_summary_attempt`) — Pending video, `unreachable()`: `RunAgainAt { later() + 6h, "can't connect to Ollama: connection refused" }`, no chats, stored video equal to the seeded one.
2. `it_should_wait_for_ollama_if_the_model_is_missing` (replaces `it_should_count_a_missing_model_as_a_failed_summary_attempt`) — `RunAgainAt { later() + 6h, "qwen3:4b isn't installed on Ollama" }`, no chats, video unchanged.
3. `it_should_keep_the_failed_attempts_of_a_retrying_summary_while_waiting_for_ollama` — summary Retrying with 10 attempts, due; `unreachable()`: re-queued 6 h, still Retrying with 10 attempts and the same `last_failed_at`.
4. `it_should_wait_for_ollama_if_it_goes_down_between_parts` — 3-part transcript, replies `[part 1 ok, Err(unreachable_error())]`: re-queued 6 h, no summary stored, video unchanged.
5. `it_should_count_a_failed_server_check_as_a_failed_summary_attempt` — `failing()`: Retrying with 1 attempt, re-queued 10 min. Expected to pass at once (regression guard).

`categorize_video_task.rs`:
6. `it_should_wait_for_ollama_if_it_is_unavailable_for_a_categorization` (replaces `it_should_count_ollama_down_as_a_failed_categorization_attempt`) — table over `unreachable()`, model missing, `Err(unreachable_error())` on the first call: each `RunAgainAt { later() + 6h, .. }`, no verdicts, categorization Pending.
7. `it_should_not_fail_pending_categories_if_ollama_is_down_on_the_eleventh_attempt` (replaces `it_should_fail_pending_categories_if_ollama_is_down_on_the_eleventh_attempt`) — Retrying with 10 attempts; table over the same three plus down during the fallback: re-queued 6 h, still Retrying with 10 attempts, no `failed` verdict; the fallback case keeps "Sicilian" `no`.
8. `it_should_wait_for_ollama_if_the_model_is_missing_with_thinking_off` — thinking off, model not listed: re-queued 6 h, no chats, categorization unchanged.
9. `it_should_count_a_connection_dropped_during_the_fallback_as_a_failed_attempt` (renames `it_should_stop_if_ollama_is_unreachable_during_the_fallback`, plain `"connection reset"` error) — Retrying with 1 attempt, re-queued 10 min. Expected to pass at once (regression guard).

2. Infrastructure tests (`ollama_llm_server_repository.rs`, real `reqwest` against local sockets)

10. `it_should_report_an_unreachable_server_on_chat` (tightens `it_should_fail_if_the_server_is_unreachable`) — closed port: `result.map_err(|e| e.is::<LlmServerUnreachable>()) == Err(true)`.
11. `it_should_report_an_unreachable_server_on_find` — closed port: `find` error is `LlmServerUnreachable`.
12. `it_should_not_report_a_dropped_connection_as_unreachable` (tightens `it_should_fail_if_the_server_drops_the_connection`) — `Err(false)`.
13. `it_should_not_report_a_slow_server_check_as_unreachable` — silent listener (accepts, never answers): `find` error is not `LlmServerUnreachable`.

The connect-timeout branch isn't tested directly (needs an unroutable address); it is the same `is_connect()` check as a refused connection.
