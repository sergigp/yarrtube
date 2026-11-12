# Design

## Files

- `src/domain/video_transcript/transcript_text.rs`: `caption_lines` replaces the per-line `strip_tags` call. It turns escape codes into what they mean, strips tags, collapses spaces, and is shared by `TranscriptText::from_srt` and `TimedTranscript`.
- `src/domain/video_transcript/timed_transcript.rs`: `cue_lines` maps each text line through `caption_lines`, since one `.srt` line can now become several.
- `src/domain/video_summary/summary_messages.rs`: `chars()`, the size the budget is checked against.
- `src/domain/video_summary/summary_input.rs`: `SummaryInput` is built with the prompt and a chars/token ratio, cuts parts by their rendered messages, and says whether reported input tokens are over the budget.
- `src/domain/services/video_summarizer.rs`: `Failure::OverBudget`. The summarizer loads the transcript once, then cuts it smaller and starts over once, at the measured ratio. A call that Ollama refuses as over the context length is an overrun too.
- `src/domain/llm_server/llm_reply.rs`: `exceeded_context`, set on a refusal because the prompt exceeds the context length.
- `src/infrastructure/repositories/ollama_llm_server_repository.rs`: a 400 whose error says the request exceeds the context becomes `LlmReply::refused_as_too_long`, since only the adapter knows Ollama's wording.
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

/// A part number at least as wide as any real one, to size the messages.
const WIDEST_PART_NUMBER: &str = "999";

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
    /// The larger of the whole call's and the widest part's messages with an
    /// empty transcript (every chapter, `WIDEST_PART_NUMBER`, a span at the
    /// video's length).
    fn prompt_chars(&self) -> usize;
    /// Shared by `part` (overlapping chapters) and `prompt_chars` (all of them).
    fn part_messages(&self, number: &str, parts: &str, part: &TranscriptPart,
        chapters: String) -> SummaryMessages;
}

// llm_reply.rs
pub struct LlmReply {
    pub content: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub stopped_at_limit: bool,
    pub refusal: Option<String>,
    /// The refusal is because the prompt exceeds the context length.
    pub exceeded_context: bool, // new
}

impl LlmReply {
    pub fn refused(reason: impl Into<String>) -> Self;            // exceeded_context: false
    pub fn refused_as_too_long(reason: impl Into<String>) -> Self; // new, exceeded_context: true
}

// video_summarizer.rs
/// A call that stopped at the context limit on more input than the budget,
/// or that Ollama refused as over the context length.
struct Overrun {
    call: String,
    input_tokens: Option<u32>, // None when refused: Ollama reports none
    chars_per_token: f64, // messages.chars() / input_tokens, or / context length when refused
    reason: anyhow::Error, // the usual "ran out of context" / "Ollama refused the call" error, if not cut again
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
    /// A pure builder: logging is `log_generating`'s.
    fn summary_input(&self, video: &Video, job: &SummaryJob,
        language: &CaptionLanguage, transcript: &TimedTranscript, chars_per_token: f64) -> SummaryInput;
    fn log_generating(video: &Video, job: &SummaryJob, input: &SummaryInput);
    fn log_cut_smaller(video: &Video, input: &SummaryInput, overrun: &Overrun);
    /// A stop at the context limit: `OverBudget` over the budget, else `Rejected`.
    fn out_of_context(call: &str, input: &SummaryInput, context_length: &ContextLength,
        message_chars: usize, input_tokens: u32) -> Failure;
    /// A refusal as over the context length: `OverBudget` at
    /// `message_chars / context_length`.
    fn exceeded_context(call: &str, context_length: &ContextLength,
        message_chars: usize, refusal: &str) -> Failure;
    /// `write_summary`; on an overrun, once more cut at its measured ratio.
    fn write_summary_cut_smaller_once(&self, video: &Video, job: &SummaryJob,
        language: &CaptionLanguage, transcript: &TimedTranscript) -> Result<SummaryText, Failure>;
    fn ask(&self, video: &Video, job: &SummaryJob, input: &SummaryInput,
        call: &str, messages: SummaryMessages) -> Result<SummaryText, Failure>; // + input
}
```

Parts are cut so that `part(i).chars() <= max_chars`. The transcript's share is `max_chars` minus the larger overhead of the whole call and of a part's `map system` + `map user` messages, each rendered with an empty transcript, every chapter and the widest part numbers (`999`). That's an upper bound for any call, so the existing cutting code stays transcript-only. Using the larger of the two keeps a transcript that doesn't fit one call from being cut into a single part.

## Call Stack

Summary (summarize task):
1. `SummarizeVideoTask::run` → `VideoSummarizer::summarize(video)`
2. → `try_generate(video, job)`
3. → `transcript(video, job)` → `(language, TimedTranscript::from_srt(srt))`. `from_srt` → `cue_lines` → `caption_lines(line)`
4. → `write_summary_cut_smaller_once(video, job, &language, &timed)`
   1. → `summary_input(.., language.chars_per_token())` → `SummaryInput::new(video, language, timed, context_length, prompt, ratio)`, then `log_generating(video, job, &input)` ("generating video summary", once per pass)
   2. → `write_summary(video, &input, job)` → `ask(video, job, &input, call, input.whole() | input.part(i) | input.reduce(..))`
      - `ask`: `reply.refusal` with `reply.exceeded_context` → `exceeded_context(call, &context_length, messages.chars(), refusal)` → `Err(Failure::OverBudget(Overrun { input_tokens: None, chars_per_token: messages.chars() / context_length, .. }))`
      - `ask`: any other refusal → `Err(Failure::Rejected(..))`, as today
      - `ask`: `reply.stopped_at_limit` → `out_of_context(call, &input, &context_length, messages.chars(), reply.input_tokens)`: over `input.over_budget(..)` → `Err(Failure::OverBudget(Overrun { input_tokens: Some(..), chars_per_token: messages.chars() / reply.input_tokens, .. }))`; within it → `Err(Failure::Rejected(..))`, as today
   3. On `Failure::OverBudget(overrun)`: `log_cut_smaller(video, &input, &overrun)` (`info!(call, input_tokens or "over the context length", budget_tokens, chars_per_token, "input over the budget, cutting the transcript smaller")`), then → `summary_input(.., overrun.chars_per_token)` → `log_generating(..)` → `write_summary(..)`. A second `OverBudget` is failed by `generate` like a rejection, with `overrun.reason`
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

11. `summarize_video_task::it_should_schedule_the_next_attempt_if_the_input_is_exactly_the_budget` (done, from the audit): a stop at the limit on exactly 9,000 input tokens is rejected, not cut again.
12. `summarize_video_task::it_should_reduce_more_than_two_answers_in_a_group_when_they_fit` (done, from the audit): a reduce group takes three answers when their messages fit.
13. `summarize_video_task::it_should_cut_smaller_when_the_prompt_exceeds_the_context`: the whole call is refused as too long (`LlmReply::refused_as_too_long`); the second pass is parts + reduce cut at the call's chars over 12,288; the summary is stored as Done.
14. `summarize_video_task::it_should_fail_the_summary_if_the_prompt_exceeds_the_context_again`: the first call of each pass is refused as too long. The summary is Retrying with one failed attempt, the reason is "Ollama refused the call: …" of the second, and nothing is stored.
15. `summarize_video_task::it_should_log_the_cut_smaller_after_a_prompt_over_the_context`: the info log names the call, `input_tokens=over the context length`, the budget and chars/token.

The existing `it_should_schedule_the_next_attempt_if_the_summary_is_rejected` (stopped at limit, 100 input tokens) already covers "Out of room within the budget", and `it_should_count_a_refused_prompt_as_a_failed_summary_attempt` (a plain refusal) covers "Refused for another reason".

Infrastructure tests (Ollama adapter, against a mock server):

1. `it_should_report_a_refusal_as_too_long_when_the_prompt_exceeds_the_context`: a 400 with `{"error": "request (7109 tokens) exceeds the available context size (512 tokens), try increasing it"}` reads as `LlmReply::refused_as_too_long(..)` (the existing `it_should_report_a_refusal_on_a_bad_request` uses this body: retarget it to another 400 error so it keeps covering a plain refusal).
2. `it_should_report_a_refusal_as_too_long_when_the_input_length_exceeds_the_context`: Ollama's other wording, `{"error": "the input length exceeds the context length"}`, reads the same way.
