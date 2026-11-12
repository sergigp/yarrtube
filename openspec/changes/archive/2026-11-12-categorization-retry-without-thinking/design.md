# Design

## Files

- `src/domain/llm_server/llm_reply.rs` — `LlmReply` gains the thinking text and `thinking_tail()`; the reply is where "how the output was spent" lives.
- `src/infrastructure/repositories/ollama_llm_server_repository.rs` — reads `message.thinking` from `/api/chat`; fake unchanged but replies carry `thinking`.
- `src/domain/services/video_categorizer.rs` — retry without thinking, give up at once, Failed-when-`failed`-verdict, new log lines.
- `src/domain/video/step_state.rs` — `give_up(now)`: Failed in one step.
- `src/domain/video/video.rs` — `give_up_categorization(now)`, the entity transition over it.
- `src/domain/services/video_summarizer.rs` — limit-stop warn line only.
- `src/domain/daemon/daemon_health.rs` — `AiStepCounts` (queued / retrying / failed) for summaries and categorizations.
- `src/domain/services/daemon_health_searcher.rs` — queued = pending tasks of that type, from the task list it already reads.
- `src/serve.rs` — heartbeat fields `summaries_queued` / `categorizations_queued`.
- `src/domain/daemon/mod.rs` — re-exports `AiStepCounts`.
- `doc/INSTALLATION.md` — the heartbeat's log fields, as users grep them.
- `src/application/tasks/categorize_video_task/tests.rs`, `src/application/tasks/summarize_video_task/tests.rs` — behaviour tests.
- `doc/AI.md` — categorization sequence, give-up, logs; `unavailable` → `failed`; summaries follow-up note.

## Types & Signatures

```rust
// domain/llm_server/llm_reply.rs
/// How much of the thinking a limit-stop log line keeps.
pub const THINKING_TAIL_CHARS: usize = 300;

pub struct LlmReply {
    pub content: String,
    /// What the model thought before answering; empty without thinking.
    pub thinking: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub stopped_at_limit: bool,
    pub refusal: Option<String>,
}
impl LlmReply {
    /// The last `THINKING_TAIL_CHARS` characters of the thinking.
    pub fn thinking_tail(&self) -> &str;
}

// domain/video/step_state.rs
impl StepState {
    /// One more failed attempt, failed `now`, and Failed whatever the count.
    pub fn give_up(self, now: DateTime<Utc>) -> Self;
}
// domain/video/video.rs
impl Video {
    pub fn give_up_categorization(self, now: DateTime<Utc>) -> Self;
}

// domain/services/video_categorizer.rs (private)
#[derive(Clone, Copy)]
enum Call { First, Fallback }                      // unchanged

/// How a call thinks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Thinking {
    /// As the job asks (settings and model).
    AsAsked,
    /// Off: a call with thinking about these categories stopped at the limit.
    Dropped,
}

/// A category without an answer from one call, with why.
struct Rejection<'a> {
    category: &'a Category,
    reason: String,
    kind: RejectionKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RejectionKind {
    /// Refused, past its timeout, unusable, or at the limit without having
    /// thought: alone in the fallback, then a later attempt.
    Retryable,
    /// Its call thought and stopped at the output limit: asked again without
    /// thinking.
    ThinkingRanOut,
    /// Rejected by a call made with `Thinking::Dropped`: alone without
    /// thinking if the call was about several, else given up now.
    WithoutThinking,
}

impl VideoCategorizer {
    fn rejected_categories<'a>(&self, video: &Video, job: &'a CategorizationJob)
        -> Result<Vec<Rejection<'a>>, Stop>;
    fn ask<'a>(&self, video: &Video, job: &CategorizationJob, categories: &'a [Category],
        call: Call, thinking: Thinking) -> Result<Vec<Rejection<'a>>, Stop>;
    /// `rejected`'s ThinkingRanOut categories asked once more, together,
    /// with `Thinking::Dropped`; the rest unchanged.
    /// `categories` asked once more, in one call with `Thinking::Dropped`,
    /// when the call about them thought and stopped at the limit (`rejected`
    /// is then every one of them); else `rejected` unchanged.
    fn retry_without_thinking<'a>(&self, video: &Video, job: &CategorizationJob,
        categories: &'a [Category], rejected: Vec<Rejection<'a>>, call: Call)
        -> Result<Vec<Rejection<'a>>, Stop>;
    fn fall_back<'a>(&self, video: &Video, job: &CategorizationJob,
        rejected: Vec<Rejection<'a>>) -> Result<Vec<Rejection<'a>>, Stop>;   // changed
    /// After a run that left nothing to retry: Pending when it gave up a
    /// category (`gave_up`) and one is still pending (its `failed` verdict was
    /// dropped: settings or prompt changed during the call), else `finish`.
    /// `gave_up` keeps a category added mid-run from resetting a normal run.
    fn settle(&self, video: &Video, job: &CategorizationJob, gave_up: bool)
        -> anyhow::Result<VideoStepOutcome>;
    /// Nothing left to ask (after a run, or `Skip::NothingPending`): Failed
    /// when the video holds a `failed` verdict, else Done.
    fn finish(&self, video: &Video) -> anyhow::Result<VideoStepOutcome>;
    fn give_up(&self, video: &Video, job: &CategorizationJob,
        rejected: &[Rejection]) -> anyhow::Result<()>;                         // takes Rejection
    /// `thinks`: whether the call asked for thinking; `retry_without_thinking`
    /// is logged as `thinking == Thinking::Dropped`.
    fn log_call(video: &Video, categories: &[Category], call: Call, thinking: Thinking,
        thinks: bool, reply: &LlmReply, started: Instant);
    fn log_limit_stop(video: &Video, categories: &[Category], thinking: bool, reply: &LlmReply);
    fn log_given_up(video: &Video, rejection: &Rejection);
}

// domain/daemon/daemon_health.rs
/// An AI step: tasks waiting for the AI, and videos retrying or failed.
pub struct AiStepCounts { pub queued: usize, pub retrying: usize, pub failed: usize }
pub struct DaemonHealth {
    // ...unchanged...
    pub downloads: StepCounts,
    pub summaries: AiStepCounts,
    pub categorizations: AiStepCounts,
}
```

## Call Stack

Categorize (`CategorizeVideoTask::run` → `VideoCategorizer::categorize(video)` → `prepare` → `ask_each(video, job)`):

```
ask_each(video, job)
  rejected_categories(video, job)
    ask(video, job, &job.pending, Call::First, Thinking::AsAsked)
      llm.chat(url, question.chat(settings, context_length, job.thinking && AsAsked))
      log_call(.., thinking, thinks, ..) → retry_without_thinking = (thinking == Dropped)
      [stopped_at_limit] log_limit_stop(..)
      -> kinds: thought & limit → ThinkingRanOut; Dropped → WithoutThinking; else Retryable
    retry_without_thinking(video, job, &job.pending, rejected, Call::First)
      [any ThinkingRanOut] ask(video, job, those, Call::First, Thinking::Dropped)
    fall_back(video, job, rejected)                 [job.pending.len() > 1]
      each rejection: log_falling_back
        ask(video, job, [category], Call::Fallback, Retryable → AsAsked | WithoutThinking → Dropped)
        retry_without_thinking(video, job, [category], that, Call::Fallback)
      remaining: log_rejected (Retryable) | log_given_up (WithoutThinking)
  log_tally(video, job, rejected.len())
  partition rejected by kind == WithoutThinking → (given_up, retryable)
  give_up(video, job, &given_up)                    // failed verdicts now, still_pending-guarded
  retryable non-empty → fail_attempt(video, job, &retryable, reason)   // today's path
  else               → settle(video, job, !given_up.is_empty())
                         gave_up && pending(video, owner) non-empty → categorization_resetter.reset  (Pending)
                         else finish(video)
                           holds a failed verdict → video_updater: give_up_categorization(now)
                           else                   → mark_categorized(video)

categorize → Skip::NothingPending → finish(video)   (no server check, no call)
```

(a) N ≥ 2, thinking: `{all, think}` limit → `{all, no-think}` → each rejected `{one, no-think}` → given up. (b) `{one, think}` limit → `{one, no-think}` → given up.

Summarize: `VideoSummarizer::ask` → `llm.chat` → `[stopped_at_limit] warn!(thinking_chars, answer_chars, thinking_tail)` → existing failure.

Heartbeat: `heartbeat_loop` → `DaemonHealthSearcher::search` → `task_repository.list_non_completed()` → `ai_step_counts(tasks, "summarize_video" | "categorize_video", ..)` (Pending tasks due now, `run_at <= now`, by `task_type()`) + `video_repository.list()` for retrying / failed → `log_heartbeat`.

## Test Plan

Behaviour (categorize task, `src/application/tasks/categorize_video_task/tests.rs`; real SQLite, fake Ollama; whole-value asserts on result, verdicts, video, scheduled tasks):

1. `it_should_ask_again_without_thinking_if_a_single_category_call_thought_past_the_limit` — chats `[{Tech, think}, {Tech, no-think}]`; `no` verdict stored; video categorization Done; `TaskOutcome::Done`.
2. `it_should_ask_every_category_again_without_thinking_if_the_call_thought_past_the_limit` — chats `[{AI,Tech, think}, {AI,Tech, no-think}]`; both verdicts.
3. `it_should_ask_alone_without_thinking_what_the_call_without_thinking_left_unanswered` — `[{Chess,Sicilian,Tech, think}, {all three, no-think}, {Tech, no-think}]`; three verdicts.
4. `it_should_ask_again_without_thinking_if_a_fallback_call_thought_past_the_limit` — `[{Sicilian,Tech, think} partly usable, {Tech, think} limit, {Tech, no-think}]`.
5. `it_should_give_up_at_once_if_the_call_without_thinking_is_rejected` — `failed` verdict for Tech; video categorization `Failed` with 1 failed attempt; no categorize task scheduled; `TaskOutcome::Done`.
6. `it_should_give_up_one_category_and_retry_another_in_the_same_run` — Tech `failed`, Food pending; Retrying with 1 attempt; `RunAgainAt` +10 min.
7. `it_should_mark_the_categorization_failed_if_a_category_was_given_up_before` — seeded Retrying + `failed` verdict for Tech; Food answered; Failed.
8. `it_should_not_give_up_a_category_if_the_settings_changed_during_its_call_without_thinking` — model changed in `on_chat` hook; no verdict for Tech; Pending, no failed attempts.
9. `it_should_retry_later_if_a_call_without_thinking_hit_the_limit` — non-thinking model: one chat, Retrying, `RunAgainAt` (replaces `it_should_not_fall_back_for_a_single_category`'s setup).
10. `it_should_log_whether_each_call_thought_and_retried_without_thinking` — the two `asked the model` lines.
11. `it_should_log_how_the_output_was_spent_when_a_call_stops_at_the_limit` — warn line with `thinking_chars`, `answer_chars`, `thinking_tail` (last 300 chars).
12. `it_should_log_a_category_given_up_at_once` — error line.

Existing tests updated, not new cycles: `it_should_ask_each_category_alone_when_the_call_hits_the_output_limit`, `it_should_not_fall_back_for_a_single_category`, `it_should_log_the_fallback` switch to a non-thinking model (their behaviour is today's, requirement 2).

12b. `it_should_mark_the_categorization_failed_without_asking_if_only_a_given_up_category_is_left` — Retrying, `failed` Tech, user verdict Food: no chat, Failed (audit fix: the nothing-pending path goes through `finish`, as `settle` does).

Behaviour (summarize task, `src/application/tasks/summarize_video_task/tests.rs`):

13. `it_should_log_how_the_output_was_spent_when_a_summary_call_stops_at_the_limit`.

Behaviour (daemon health, `src/domain/services/daemon_health_searcher.rs` tests, where the heartbeat counts are already tested):

14. `it_should_count_ai_tasks_due_not_pending_videos` — 3 Pending-summary videos, 1 pending summarize task, 2 pending + 1 running + 1 deferred (24 h) categorize tasks → `queued` 1 and 2. Existing `it_should_report_queue_and_library_counts` updated to `AiStepCounts`.

Infrastructure (Ollama adapter, `ollama_llm_server_repository.rs`, against mockito):

15. `it_should_read_the_thinking` — `message.thinking` → `LlmReply.thinking`; absent → empty.
