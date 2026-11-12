# Design

## Files

- `src/domain/video_transcript/transcript_text.rs`: `caption_lines` replaces the per-line `strip_tags` call. It turns escape codes into what they mean, strips tags, collapses spaces, and is shared by `TranscriptText::from_srt` and `TimedTranscript`.
- `src/domain/video_transcript/timed_transcript.rs`: `cue_lines` maps each text line through `caption_lines`, since one `.srt` line can now become several.
- `src/domain/video_summary/summary_messages.rs`: `chars()`, the size the budget is checked against.
- `src/domain/video_summary/summary_input.rs`: `SummaryInput` is built with the prompt and a chars/token ratio, cuts parts by their rendered messages, and says whether reported input tokens are over the budget.
- `src/domain/services/video_summarizer.rs`: `Failure::OverBudget`. The summarizer loads the transcript once, then cuts it smaller and starts over once, at the measured ratio.
- `src/application/tasks/fetch_subtitles_task/tests.rs`, `src/application/tasks/summarize_video_task/tests.rs`: acceptance tests.
- `doc/AI.md`: "How a summary is generated".

## Types & Signatures

```rust
// transcript_text.rs
/// `line`'s caption text: `\N` splits it, `\h` is a space, tags stripped,
/// spaces collapsed, each trimmed, empty ones dropped.
pub(super) fn caption_lines(line: &str) -> Vec<String>;

// summary_messages.rs
impl SummaryMessages {
    /// System and user together, in characters.
    pub fn chars(&self) -> usize;
}

// summary_input.rs
pub struct SummaryInput {
    title: String,
    minutes: i64,
    language: String,
    chapters: Vec<VideoChapter>,
    parts: Vec<TranscriptPart>,
    prompt: SummaryPrompt,
    budget_tokens: u32,
    max_chars: usize, // budget_tokens * chars_per_token: whole messages, not transcript
}

impl SummaryInput {
    pub fn new(
        video: &Video,
        language: &CaptionLanguage,
        transcript: &TimedTranscript,
        context_length: &ContextLength,
        prompt: &SummaryPrompt,
        chars_per_token: f64,
    ) -> Self;
    pub fn parts(&self) -> usize;
    pub fn whole(&self) -> SummaryMessages;                 // prompt param dropped
    pub fn part(&self, index: usize) -> SummaryMessages;    // prompt param dropped
    pub fn answers(&self, summaries: Vec<SummaryText>) -> Vec<PartAnswer>;
    pub fn reduce(&self, answers: &[PartAnswer]) -> SummaryMessages;
    pub fn reduce_group(&self, answers: &[PartAnswer]) -> SummaryMessages;
    pub fn reduce_fits(&self, answers: &[PartAnswer]) -> bool;      // reduce(answers).chars() <= max_chars
    pub fn reduce_groups(&self, answers: &[PartAnswer]) -> Vec<Vec<PartAnswer>>; // reduce_group(..).chars()
    /// Whether `input_tokens`, as Ollama reported them, are over the budget.
    pub fn over_budget(&self, input_tokens: u32) -> bool;
    pub fn budget_tokens(&self) -> u32;
}

// video_summarizer.rs
/// A call that stopped at the context limit on more input than the budget.
struct Overrun {
    call: String,
    input_tokens: u32,
    chars_per_token: f64, // messages.chars() / input_tokens
    reason: anyhow::Error, // the usual "ran out of context" error, if not cut again
}

enum Failure {
    Unavailable(anyhow::Error),
    Server(anyhow::Error),
    Rejected(anyhow::Error),
    OverBudget(Overrun), // new
}

impl VideoSummarizer {
    fn transcript(&self, video: &Video, job: &SummaryJob)
        -> anyhow::Result<(CaptionLanguage, TimedTranscript)>; // was summary_input
    fn summary_input(&self, video: &Video, job: &SummaryJob,
        language: &CaptionLanguage, transcript: &TimedTranscript, chars_per_token: f64) -> SummaryInput;
    /// `write_summary`; on an overrun, once more cut at its measured ratio.
    fn write_summary_cut_smaller_once(&self, video: &Video, job: &SummaryJob,
        language: &CaptionLanguage, transcript: &TimedTranscript) -> Result<SummaryText, Failure>;
    fn ask(&self, video: &Video, job: &SummaryJob, input: &SummaryInput,
        call: &str, messages: SummaryMessages) -> Result<SummaryText, Failure>; // + input
}
```

Parts are cut so that `part(i).chars() <= max_chars`. The transcript's share is `max_chars` minus the overhead of the `map system` + `map user` messages, rendered with an empty transcript, every chapter and the widest part numbers. That's an upper bound for any part, so the existing cutting code stays transcript-only. Whole: `whole().chars() <= max_chars` means one part.

## Call Stack

Summary (summarize task):
1. `SummarizeVideoTask::run` → `VideoSummarizer::summarize(video)`
2. → `try_generate(video, job)`
3. → `transcript(video, job)` → `(language, TimedTranscript::from_srt(srt))`. `from_srt` → `cue_lines` → `caption_lines(line)`
4. → `write_summary_cut_smaller_once(video, job, &language, &timed)`
   1. → `summary_input(.., language.chars_per_token())` → `SummaryInput::new(video, language, timed, context_length, prompt, ratio)`
   2. → `write_summary(video, &input, job)` → `ask(video, job, &input, call, input.whole() | input.part(i) | input.reduce(..))`
      - `ask`: `reply.stopped_at_limit && input.over_budget(reply.input_tokens)` → `Err(Failure::OverBudget(Overrun { chars_per_token: messages.chars() / reply.input_tokens, .. }))`
      - `ask`: `stopped_at_limit` within budget → `Err(Failure::Rejected(..))`, as today
   3. On `Failure::OverBudget(overrun)`: `info!(call, input_tokens, budget_tokens, chars_per_token, "input over the budget, cutting the transcript smaller")`, then → `summary_input(.., overrun.chars_per_token)` → `write_summary(..)`. A second `OverBudget` → `Failure::Rejected(overrun.reason)`
5. → store the summary, as today

Transcript stored (fetch subtitles task):
1. `FetchSubtitlesTask::run` → `SubtitleFetcher` → `TranscriptText::from_srt(srt)` → `caption_lines(line)` per cue text line

## Test Plan

Behaviour tests (application layer, tasks with real SQLite + `FakeLlmServerRepository` + fake files):

1. `fetch_subtitles_task::it_should_store_hard_spaces_as_spaces_in_the_transcript`: a track with `play the Jabava\h` / `London\h\h` stores the lines `play the Jabava` and `London`.
2. `fetch_subtitles_task::it_should_store_a_forced_line_break_as_a_line_break`: `first line\Nsecond\h\hline` stores `first line` and `second line`.
3. `summarize_video_task::it_should_send_the_transcript_without_hard_spaces`: the `.srt` lines with `\h` reach the user message clean.
4. `summarize_video_task::it_should_cut_in_parts_when_the_prompt_pushes_the_call_over_the_budget`: a Spanish transcript of 30,000 chars (under 30,600 alone) is sent as parts + reduce, not whole.
5. `summarize_video_task::it_should_keep_every_part_call_within_the_budget`: a chaptered transcript cut by size; the exact part messages are asserted, each part's messages `chars()` ≤ 30,600.
6. `summarize_video_task::it_should_reduce_in_groups_when_the_reduce_prompt_pushes_the_answers_over_the_budget`: the answers fit 30,600 alone but not inside the reduce messages; group reduces come first.
7. `summarize_video_task::it_should_cut_smaller_when_a_part_runs_over_the_budget`: the first part replies `stopped_at_limit` with input tokens over 9,000. The calls that follow are the transcript cut at the measured ratio (more parts), all made again, and the summary is stored as Done.
8. `summarize_video_task::it_should_cut_in_parts_when_the_whole_transcript_runs_over_the_budget`: the whole call overruns; the second pass is parts + reduce; the summary is stored.
9. `summarize_video_task::it_should_fail_the_summary_if_the_input_runs_over_the_budget_again`: the first call of each pass overruns. The summary is Retrying with one failed attempt, the reason is "the model ran out of context (…)" of the second overrun, and nothing is stored.
10. `summarize_video_task::it_should_log_the_cut_smaller_with_the_measured_ratio`: the info log names the call, input tokens, budget and chars/token.

The existing `it_should_schedule_the_next_attempt_if_the_summary_is_rejected` (stopped at limit, 100 input tokens) already covers "Out of room within the budget".

Infrastructure tests: none. No adapter changes.
