## MODIFIED Requirements

### Requirement: Prompt Variables Filled
Each message SHALL be the prompt's section with its variables replaced in one pass, so braces inside a value stay as they are: `{title}`, `{minutes}` (whole minutes), `{lang}` (the transcript language's English name, or its code when the name isn't known) and `{transcript}`. `{transcript}` SHALL be the subtitle file's captions cleaned as the stored transcript is (see "Transcript Stored" in `video-subtitles`), subtitle escape codes included.

#### Scenario: Filling the prompt
- **WHEN** a 30-minute video titled `Sicilian` has a `es-US` transcript
- **THEN** the system message comes from the `system` section with `{lang}` as `Spanish`, and the user message from `user` with the title, `30`, `Spanish` and the transcript

#### Scenario: Value containing braces
- **WHEN** a video's title is `{transcript} explained`
- **THEN** the user message contains the title unchanged

#### Scenario: Unknown language
- **WHEN** a transcript's language is `xh`, whose name isn't known
- **THEN** `{lang}` is `xh`

#### Scenario: Subtitle file with hard spaces
- **WHEN** a video's subtitle file has the lines `play the Jabava\h` and `London\h\h`
- **THEN** the transcript in the user message has the lines `play the Jabava` and `London`

### Requirement: Input Budget
A transcript SHALL be summarized in one call when the estimated size of that call's whole messages (system and user, with the title, duration, chapters and transcript filled in) fits the input budget, about 73% of the context length (9000 tokens at the default 12288). Size is estimated at 4.3 characters per token for English, 1.5 for Japanese, Chinese and Korean, and 3.4 for other languages.

#### Scenario: Transcript fits
- **WHEN** the context length is 12288 and a Spanish transcript has 20000 characters
- **THEN** one call is made, with the `system` and `user` sections

#### Scenario: Japanese transcript
- **WHEN** the context length is 12288 and a Japanese transcript has 20000 characters
- **THEN** the transcript is summarized in parts

#### Scenario: Smaller context length
- **WHEN** the context length is 6144 and a Spanish transcript has 20000 characters
- **THEN** the transcript is summarized in parts

#### Scenario: Prompt pushes the call over the budget
- **WHEN** the context length is 12288 and a Spanish transcript has 30000 characters, under the 30600 the budget allows on its own but over it with the shipped prompt's messages around it
- **THEN** the transcript is summarized in parts

### Requirement: Long Transcripts Summarized In Parts
A transcript over the budget SHALL be cut into consecutive parts whose calls' whole messages (system and user, with the part's header, chapters and text filled in) each fit the budget, at chapter starts when the video has chapters, packing whole chapters together and cutting by size only a chapter too long on its own. Each part SHALL be summarized with `map system` and `map user`, then the parts' answers with `reduce system` and `reduce user`.

#### Scenario: Chaptered video
- **WHEN** a 3-hour video has 6 chapters and its transcript is 3 times the budget
- **THEN** every part starts at a chapter start and the last call uses the `reduce` sections

#### Scenario: Video without chapters
- **WHEN** a 3-hour video has no chapters and its transcript is 3 times the budget
- **THEN** it is cut by size into parts under the budget

#### Scenario: One chapter over the budget
- **WHEN** one chapter's text alone is over the budget
- **THEN** that chapter is cut by size and the others stay whole

#### Scenario: Part messages within the budget
- **WHEN** a transcript is cut into parts
- **THEN** every part's system and user messages together are estimated within the budget

### Requirement: Answers Reduced In Groups
When the reduce call's whole messages (system and user, with the chapters and every answer filled in through `reduce part`) don't fit the budget, consecutive answers SHALL first be reduced in groups of at least two whose calls' whole messages fit it, without the chapters, and the results reduced the same way, until they fit or one answer is left. The last reduce SHALL list the chapters.

#### Scenario: Answers over the budget
- **WHEN** the answers of 30 parts don't fit the budget together
- **THEN** groups of them are reduced first, and the last call reduces those results and lists the chapters

#### Scenario: Reduce prompt pushes the answers over the budget
- **WHEN** the answers alone fit the budget but not with the reduce prompt's messages around them
- **THEN** groups of them are reduced first

### Requirement: Incomplete Answers Rejected
A call SHALL be rejected when its answer is empty once cleaned, when Ollama stopped at the context limit (unless its input ran over the budget for the first time in this generation, see "Input Over The Budget Cut Smaller"), when Ollama refuses the prompt as too long, or when it runs past the 30-minute timeout. A rejected call SHALL fail the whole generation, storing nothing.

#### Scenario: Stopped at the output cap
- **WHEN** Ollama reports it stopped because of the length limit, with input tokens within the budget
- **THEN** no summary is stored and the generation fails

#### Scenario: Prompt too long
- **WHEN** Ollama answers 400 because the prompt exceeds the context length
- **THEN** no summary is stored and the generation fails

#### Scenario: Reasoning only
- **WHEN** Ollama's answer is only `<think>hmm</think>`
- **THEN** no summary is stored and the generation fails

#### Scenario: Part fails
- **WHEN** the second of 3 parts fails
- **THEN** no further calls are made, no summary is stored and the generation fails

### Requirement: Summary Generation Logged
Generation SHALL be logged at info level from start to end: the start with the video and its length in seconds; each call with its full system and user messages, then its full answer, tokens and duration; a re-cut after an input over the budget with the call, its input tokens, the budget and the characters per token measured; and the end with the summary, total duration, video length and model. Skips SHALL be logged with their reason at info level, and failures at error level with the specific reason: the context, input tokens and room left when the context ran out, or Ollama's refusal.

#### Scenario: Successful generation
- **WHEN** a 30-minute video is summarized in one call at info verbosity
- **THEN** the logs hold the video's length, the full messages sent, the answer, the call's duration, and a final entry with the summary, the total duration and the video's length

#### Scenario: Summarized in parts
- **WHEN** a video is summarized in 3 parts
- **THEN** each of the 4 calls logs its own messages, answer and duration

#### Scenario: Skipped
- **WHEN** a summary task runs for a source that opted out
- **THEN** the logs say the video was skipped and why, at info level

#### Scenario: Failed
- **WHEN** a generation fails
- **THEN** the logs hold the specific reason, at error level

#### Scenario: Context ran out
- **WHEN** a call with 100 input tokens fills a context of 12288
- **THEN** the error logged names the context of 12288, the 100 input tokens and the 12188 tokens left

#### Scenario: Cut smaller
- **WHEN** a call with 11932 input tokens on a budget of 9000 stops at the context limit and the generation is cut again
- **THEN** the logs name the call, the 11932 input tokens, the 9000 budget and the characters per token measured, at info level

## ADDED Requirements

### Requirement: Input Over The Budget Cut Smaller
When a call stops at the context limit and Ollama reports more input tokens than the budget, the generation SHALL start over once, in the same run, with sizes estimated at that call's measured characters per token (its messages' characters over its input tokens) instead of the language's. The earlier answers SHALL be discarded. A second overrun, or a stop at the limit with input within the budget, SHALL be rejected.

#### Scenario: Part over the budget
- **WHEN** a transcript's first part stops at the context limit reporting 11932 input tokens on a budget of 9000
- **THEN** the transcript is cut again into smaller parts estimated at that part's measured characters per token, every call is made again, and the summary is stored

#### Scenario: Whole transcript over the budget
- **WHEN** a transcript sent in one call stops at the context limit reporting more input tokens than the budget
- **THEN** it is summarized in parts estimated at that call's measured characters per token, and the summary is stored

#### Scenario: Over the budget again
- **WHEN** a call of the second pass also stops at the context limit reporting more input tokens than the budget
- **THEN** no summary is stored and the generation fails

#### Scenario: Out of room within the budget
- **WHEN** a call stops at the context limit reporting input tokens within the budget
- **THEN** it is not cut again: no summary is stored and the generation fails
