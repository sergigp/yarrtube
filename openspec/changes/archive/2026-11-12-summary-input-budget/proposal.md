# Proposal

## Why

A summary's input can overrun its budget until the model has no room left to answer. In prod, the chess video `eef9c810` failed 4 times on its map call "part 1 of 2": 11,932 input tokens on a 12,288 context, against a 9,000-token budget. A local reproduction with `qwen3:4b` gives the same 11,932. The causes:

- **`\h` escapes.** These are subtitle hard spaces, left at almost every line end in the `.srt`. They survive caption cleaning and cost about 2.2k tokens on that part. They appear in 27 of 241 manual and 4 of 417 auto-generated transcripts in prod.
- **Content denser than the fixed ratio.** Clean chess English runs at 3.75 chars/token against the 4.3 assumed for English.
- **The budget counts only the transcript.** The prompt, title and chapters around it (about 220 tokens here) are left out.

Every retry sends the same input, so the remaining attempts would all fail the same way.

## What Changes

- Caption cleaning turns subtitle escape codes into what they mean: `\h` (hard space) becomes a space and `\N` (forced line break) a line break, with runs of spaces collapsed. This applies to the stored transcript and to the timed lines the summary reads from the `.srt`.
- The input budget covers each call's whole messages (system, user template, title, duration, chapters, part header, reduce templates), not just the transcript or the part answers.
- When a call stops at the context limit and Ollama reports more input tokens than the budget, the generation is cut again, once, within the same run. The new cut uses the chars per token measured on that call, and the summary starts over with it. It's rejected as before only if the second pass overruns too, or if the input was within budget (the model ran out of room thinking).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `video-subtitles`: Transcript Stored. Subtitle escape codes `\h` and `\N` become a space and a line break instead of staying as text.
- `video-summaries`:
  - Input Budget counts the whole messages.
  - Long Transcripts Summarized In Parts and Answers Reduced In Groups are budgeted on whole messages.
  - New requirement: an input that ran over the budget is cut smaller once, at the measured ratio, before the call is rejected.
  - Incomplete Answers Rejected gains that exception.
  - Summary Generation Logged logs the re-cut.

## Impact

- Code:
  - `src/domain/video_transcript/transcript_text.rs`: caption line cleaning, shared by `TranscriptText` and `TimedTranscript`.
  - `src/domain/video_summary/summary_input.rs`: budget on rendered messages, chars per token passed in.
  - `src/domain/video_summary/summary_messages.rs`: message size.
  - `src/domain/services/video_summarizer.rs`: overrun detection and a single re-cut.
- Tests:
  - `src/application/tasks/summarize_video_task/tests.rs`
  - `src/application/tasks/fetch_subtitles_task/tests.rs`
- Docs: `doc/AI.md` ("How a summary is generated").
- Stored transcripts already in the database keep their `\h` until fetched again. Summaries are unaffected, because they read the timed lines from the `.srt`.
- Out of scope:
  - thinking defaults
  - categorization (branch `output-limit-retry-without-thinking`)
  - the 2 Failed summaries from 5 Nov, which hit the old 3,288-token output cap and fit the budget
