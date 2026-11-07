## Context

`shift: false` already makes Ollama stop at the end of the context with `done_reason: length`. `num_predict` only adds a lower limit, and that limit is the failure (see proposal.md).

## Goals / Non-Goals

**Goals:** a call's output is limited only by the free context; failures say how much room there was.

**Non-Goals:** changing the input budget, the timeout or the retries.

## Decisions

- `LlmChat.max_output_tokens` stays and is set to the context length. It's a bound if an Ollama version ignores `shift`; don't remove it as redundant.

## Risks / Trade-offs

- Prompts can exceed the budget: production logged 11,131 input tokens against a 9000 budget, as the budget counts only transcript characters. Such a call has ~1,150 tokens left and can still fail → the new error shows how often; retune the budget from those logs in a separate change.
- Runaway thinking lasts longer per attempt (up to ~11 min on a short input) → 30-minute timeout still applies.

## Files

- `src/domain/preferences/context_length.rs`: remove `output_budget_tokens`; the input budget stays.
- `src/domain/services/video_summarizer.rs`: send the context length as the limit; new error text.
- `src/application/tasks/summarize_video_task.rs`: behaviour tests.
- `doc/AI.md`: "output cap" becomes the context limit.
- `evals/summaries/decisions.md`: qwen3 output range up to 3,551; `num_predict` gotcha.

## Types & Signatures

```rust
impl ContextLength {
    // removed: pub fn output_budget_tokens(&self) -> u32
    pub fn input_budget_tokens(&self) -> u32; // unchanged
}

pub struct LlmChat {
    // unchanged field, now always the context length
    pub max_output_tokens: u32,
}
```

## Call Stack

```
SummarizeVideoTask::handle(payload, is_last_attempt)
  VideoSummarizer::summarize(video, is_last_attempt)
    VideoSummarizer::ask(video, job, call, messages)
      LlmServerRepository::chat(url, LlmChat { context_length, max_output_tokens: context_length, .. })
      reply.stopped_at_limit -> Failure::Rejected(context, input_tokens, context - input_tokens)
```

## Test Plan

1. Behaviour tests (`summarize_video_task.rs`):
   - `it_should_send_the_saved_model_settings` (existing): `max_output_tokens` equals the saved context length, 8192.
   - `it_should_fail_the_task_when_the_answer_stopped_at_the_limit` (existing): error reads `the model ran out of context (12288 tokens: 100 of input, 12188 left to think and answer)`.
2. Infrastructure tests: none, the adapter doesn't change.
