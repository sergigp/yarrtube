## Why

Summaries fail at a fixed output cap of 3288 tokens (context minus input budget) even when most of the context is free: production logged 17 failures on 10 videos in one day (2026-11-04/05), with inputs as low as 2,139 tokens, about 10k of context unused. The cap is also below normal use: the prompt v3 eval, which ran uncapped, measured `qwen3:4b` writing up to 3,551 tokens on a 2,447-token input. Each failure repeats a call of minutes, and a video that fails 5 times becomes Unavailable.

## What Changes

- A summary call can use all the context its prompt leaves: the fixed output cap is gone.
- A call that fills the context is still rejected, and its error reports the context, the input tokens and the room that was left.
- The 30-minute timeout stays as the guard against runaway thinking.
- The input budget doesn't change, so a prompt near the context size still has little room to answer (see design.md, Risks).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `video-summaries`: requests no longer cap output at what the input budget leaves; rejection and failure logging speak of the context limit instead of an output cap.

## Impact

- `src/domain/preferences/context_length.rs`, `src/domain/services/video_summarizer.rs`: the cap and the error.
- `doc/AI.md`, `evals/summaries/decisions.md`: output-cap wording and the measured output range.
- No API, UI, schema or settings change. Runaway thinking on a short input can now run up to about 10k tokens (about 11 min at 15 tok/s) per attempt, instead of 3288.
