# Spec Delta

## Purpose

Generates a short AI summary of each video from its transcript, through the user's Ollama server, for the sources that opted in, and stores it for later features to show.

## ADDED Requirements

### Requirement: Summary State
Every stored video SHALL have a summary state: Missing, Generated or Unavailable. Videos stored before this state existed SHALL be Missing. Generated and Unavailable are final.

#### Scenario: Existing videos after upgrade
- **WHEN** the daemon starts on a database with videos stored before summaries existed
- **THEN** every one of them has summary state Missing and no recorded check

### Requirement: Summary Generation Conditions
A summary SHALL be generated for a video only when, at the moment it runs, AI and summaries are on, the video's channel or playlist has `summaries_enabled` `true`, the video is Downloaded, its subtitles are Fetched and its summary state is Missing. Otherwise the run SHALL do nothing and change nothing.

#### Scenario: Source opted out after the task was queued
- **WHEN** a summary task runs for a video whose channel has since turned `summaries_enabled` off
- **THEN** Ollama is not called and the video stays Missing

#### Scenario: AI turned off after the task was queued
- **WHEN** a summary task runs while AI is off
- **THEN** Ollama is not called and the video stays Missing

#### Scenario: Video already summarized
- **WHEN** a summary task runs for a video whose summary state is Generated
- **THEN** Ollama is not called and the stored summary is unchanged

### Requirement: Summary Prompt Used
Generation SHALL use the saved prompt, or the prompt shipped with the running version when none is saved. A saved prompt that no longer passes prompt validation SHALL stop the generation with a logged warning, without counting a failed attempt.

#### Scenario: Prompt never saved
- **WHEN** no prompt was saved and a summary is generated
- **THEN** the messages sent to Ollama come from the shipped prompt

#### Scenario: Saved prompt
- **WHEN** a prompt was saved and a summary is generated
- **THEN** the messages sent to Ollama come from the saved prompt

#### Scenario: Saved prompt invalid under newer rules
- **WHEN** the saved prompt is missing a section the running version requires
- **THEN** Ollama is not asked to summarize, no failed attempt is counted and the video stays Missing

### Requirement: Prompt Variables Filled
Each message SHALL be the prompt's section with its variables replaced in one pass, so braces inside a value stay as they are: `{title}`, `{minutes}` (whole minutes), `{lang}` (the transcript language's English name, or its code when the name isn't known) and `{transcript}`.

#### Scenario: Filling the prompt
- **WHEN** a 30-minute video titled `Sicilian` has a `es-US` transcript
- **THEN** the system message comes from the `system` section with `{lang}` as `Spanish`, and the user message from `user` with the title, `30`, `Spanish` and the transcript

#### Scenario: Value containing braces
- **WHEN** a video's title is `{transcript} explained`
- **THEN** the user message contains the title unchanged

#### Scenario: Unknown language
- **WHEN** a transcript's language is `xh`, whose name isn't known
- **THEN** `{lang}` is `xh`

### Requirement: Chapters In The Prompt
For a video with chapters, `{chapters}` SHALL be the `chapters` section with `{chapter_list}` listing each chapter as `<start> <title>`, on lines of its own. For a video without chapters, `{chapters}` SHALL be empty.

#### Scenario: Video with chapters
- **WHEN** a video has chapters `Intro` at 0:00 and `Game 1` at 2:05
- **THEN** the user message contains the `chapters` section listing `0:00 Intro` and `2:05 Game 1`

#### Scenario: Video without chapters
- **WHEN** a video has no chapters
- **THEN** nothing is inserted for `{chapters}`

### Requirement: Summary Settings Sent To Ollama
Every request SHALL use the saved `summaries_model`, `summaries_context_length` and `summaries_temperature`, and cap the output at the context left over by the input budget. Thinking SHALL be requested only when `summaries_thinking` is on and Ollama lists `thinking` among the model's capabilities.

#### Scenario: Model able to think
- **WHEN** thinking is on and the saved model supports it
- **THEN** requests ask for thinking

#### Scenario: Model unable to think
- **WHEN** thinking is on and Ollama doesn't list `thinking` for the saved model
- **THEN** requests don't ask for thinking and the summary is generated

### Requirement: Input Budget
A transcript SHALL be summarized in one call when its estimated size fits the input budget, about 73% of the context length (9000 tokens at the default 12288). Size is estimated at 4.3 characters per token for English and 3.4 for other languages.

#### Scenario: Transcript fits
- **WHEN** the context length is 12288 and a Spanish transcript has 20000 characters
- **THEN** one call is made, with the `system` and `user` sections

#### Scenario: Smaller context length
- **WHEN** the context length is 6144 and a Spanish transcript has 20000 characters
- **THEN** the transcript is summarized in parts

### Requirement: Long Transcripts Summarized In Parts
A transcript over the budget SHALL be cut into consecutive parts under it, at chapter starts when the video has chapters, packing whole chapters together and cutting by size only a chapter too long on its own. Each part SHALL be summarized with `map system` and `map user`, then the parts' answers with `reduce system` and `reduce user`.

#### Scenario: Chaptered video
- **WHEN** a 3-hour video has 6 chapters and its transcript is 3 times the budget
- **THEN** every part starts at a chapter start and the last call uses the `reduce` sections

#### Scenario: Video without chapters
- **WHEN** a 3-hour video has no chapters and its transcript is 3 times the budget
- **THEN** it is cut by size into parts under the budget

#### Scenario: One chapter over the budget
- **WHEN** one chapter's text alone is over the budget
- **THEN** that chapter is cut by size and the others stay whole

### Requirement: Part Variables Filled
A part's message SHALL fill `{part}`, `{parts}`, `{span}` (the times of its first and last line, as `<start>–<end>`), `{transcript}` with that part's text and `{chapters}` from `map chapters`, listing only the chapters overlapping the part. Each answer SHALL be added to `{part_summaries}` through `reduce part`, in order.

#### Scenario: Second part of three
- **WHEN** the second of 3 parts runs from 45:00 to 1:30:10
- **THEN** its user message has part `2` of `3`, span `45:00–1:30:10` and only the chapters overlapping that span

#### Scenario: Reduce
- **WHEN** all 3 parts are answered
- **THEN** the reduce user message holds the 3 answers in order, each filled through `reduce part`

### Requirement: Answer Cleaned
Everything up to and including the last `</think>` in an answer SHALL be dropped, and the result trimmed, for part answers and the final one, because some models write their reasoning into the answer even when told not to.

#### Scenario: Reasoning in the answer
- **WHEN** Ollama answers `<think>hmm</think>\n\nThree games.`
- **THEN** the answer used is `Three games.`

### Requirement: Incomplete Answers Rejected
A call SHALL count as failed when its answer is empty once cleaned, when Ollama stopped at the output cap, or when the input reached within 64 tokens of the context length, since Ollama truncates input silently. A failed call SHALL fail the whole generation, storing nothing.

#### Scenario: Stopped at the output cap
- **WHEN** Ollama reports it stopped because of the length limit
- **THEN** no summary is stored and one failed attempt is counted

#### Scenario: Input truncated
- **WHEN** Ollama reports 12250 input tokens with a context length of 12288
- **THEN** no summary is stored and one failed attempt is counted

#### Scenario: Reasoning only
- **WHEN** Ollama's answer is only `<think>hmm</think>`
- **THEN** no summary is stored and one failed attempt is counted

#### Scenario: Part fails
- **WHEN** the second of 3 parts fails
- **THEN** no further calls are made, no summary is stored and one failed attempt is counted

### Requirement: Summary Stored
A successful generation SHALL store the summary with the model that wrote it and when, and set the summary state to Generated. A video SHALL have at most one summary. A stored summary SHALL NOT be regenerated, including when the prompt or model changes or the video is redownloaded, and SHALL be kept when summaries are turned off.

#### Scenario: Summary generated
- **WHEN** Ollama answers with a summary
- **THEN** it is stored for the video with the model's name and the video's summary state is Generated

#### Scenario: Prompt changed later
- **WHEN** a video is Generated and the saved prompt changes
- **THEN** the video is not summarized again

#### Scenario: Video deleted during generation
- **WHEN** a video's record is deleted while its summary is being generated
- **THEN** nothing is stored

### Requirement: Ollama Unavailable
When Ollama can't be reached before generating, or the saved model isn't installed on it, the generation SHALL stop with a logged warning, recording no check and no failed attempt, so a later sync tries again.

#### Scenario: Ollama down
- **WHEN** a summary task runs while nothing answers at the Ollama URL
- **THEN** the video stays Missing, no check is recorded and the task completes

#### Scenario: Model not installed
- **WHEN** the saved model isn't among those Ollama lists
- **THEN** the video stays Missing, no check is recorded and the task completes

### Requirement: Failed Summaries Bounded
A generation that fails after Ollama was reached, or whose transcript can't be read, SHALL record a check and count a failed attempt. The third failed attempt SHALL set the summary state to Unavailable. The task itself SHALL complete, never retried by the task queue.

#### Scenario: First failure
- **WHEN** a generation fails for a video with no failed attempts
- **THEN** the video stays Missing with 1 failed attempt and its check time recorded

#### Scenario: Third failure
- **WHEN** a generation fails for a video with 2 failed attempts
- **THEN** the video's summary state is Unavailable

#### Scenario: Subtitle file missing
- **WHEN** a video's recorded subtitle file is no longer on disk
- **THEN** Ollama is not asked to summarize and one failed attempt is counted

### Requirement: Summary Deleted With Its Video
A video's summary SHALL be deleted whenever the video's record is deleted, including when its whole playlist or channel is deleted.

#### Scenario: Video removed
- **WHEN** a video with a stored summary is removed from its playlist or channel and its record deleted
- **THEN** its summary is deleted too

#### Scenario: Playlist deleted
- **WHEN** a tracked playlist whose videos have stored summaries is deleted
- **THEN** those summaries are deleted too

### Requirement: Summary Generation Logged
Generation SHALL be logged at info level from start to end: the start with the video and its length in seconds; each call with its full system and user messages, then its full answer, tokens and duration; and the end with the summary, total duration, video length and model. Skips SHALL be logged with their reason at info level, and failures at warn level.

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
- **THEN** the logs hold the reason, at warn level, with the failed attempts so far
