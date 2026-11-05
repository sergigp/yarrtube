# video-summaries Specification

## Purpose

Generates a short AI summary of each video from its transcript, through the user's Ollama server, for the sources that opted in, stores it, and decides when it may be shown.

## Requirements

### Requirement: Summary State
Every stored video SHALL have a summary state: Missing, Generated or Unavailable. Videos stored before this state existed SHALL be Missing. Generated is final. Unavailable lasts until the summary settings change.

#### Scenario: Existing videos after upgrade
- **WHEN** the daemon starts on a database with videos stored before summaries existed
- **THEN** every one of them has summary state Missing

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
Generation SHALL use the saved prompt, or the prompt shipped with the running version when none is saved. A saved prompt that no longer passes prompt validation SHALL stop the generation with a logged warning, changing nothing, and the task SHALL complete.

#### Scenario: Prompt never saved
- **WHEN** no prompt was saved and a summary is generated
- **THEN** the messages sent to Ollama come from the shipped prompt

#### Scenario: Saved prompt
- **WHEN** a prompt was saved and a summary is generated
- **THEN** the messages sent to Ollama come from the saved prompt

#### Scenario: Saved prompt invalid under newer rules
- **WHEN** the saved prompt is missing a section the running version requires
- **THEN** Ollama is not asked to summarize, the task completes and the video stays Missing

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
Every request SHALL use the saved `summaries_model`, `summaries_context_length` and `summaries_temperature`, and SHALL let the answer use all the context the prompt leaves, with no lower output cap. Thinking SHALL be requested only when `summaries_thinking` is on and Ollama lists `thinking` among the model's capabilities. Every request SHALL tell Ollama neither to truncate the prompt nor to shift the context, because by default it silently cuts an oversized prompt and reports only what it kept.

#### Scenario: Never truncated
- **WHEN** any request is sent
- **THEN** it asks Ollama not to truncate and not to shift

#### Scenario: All the free context available
- **WHEN** a request is sent with a context length of 8192
- **THEN** its output limit is 8192, leaving Ollama to stop where the context ends

#### Scenario: Model able to think
- **WHEN** thinking is on and the saved model supports it
- **THEN** requests ask for thinking

#### Scenario: Model unable to think
- **WHEN** thinking is on and Ollama doesn't list `thinking` for the saved model
- **THEN** requests don't ask for thinking and the summary is generated

### Requirement: Input Budget
A transcript SHALL be summarized in one call when its estimated size fits the input budget, about 73% of the context length (9000 tokens at the default 12288). Size is estimated at 4.3 characters per token for English, 1.5 for Japanese, Chinese and Korean, and 3.4 for other languages.

#### Scenario: Transcript fits
- **WHEN** the context length is 12288 and a Spanish transcript has 20000 characters
- **THEN** one call is made, with the `system` and `user` sections

#### Scenario: Japanese transcript
- **WHEN** the context length is 12288 and a Japanese transcript has 20000 characters
- **THEN** the transcript is summarized in parts

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

### Requirement: Answers Reduced In Groups
When the answers and the chapters don't fit the budget together, consecutive answers SHALL first be reduced in groups of at least two, without the chapters, and the results reduced the same way, until they fit or one answer is left. The last reduce SHALL list the chapters.

#### Scenario: Answers over the budget
- **WHEN** the answers of 30 parts don't fit the budget together
- **THEN** groups of them are reduced first, and the last call reduces those results and lists the chapters

### Requirement: Part Variables Filled
A part's message SHALL fill `{part}`, `{parts}`, `{span}` (the times of its first and last line, as `<start>–<end>`), `{transcript}` with that part's text and `{chapters}` from `map chapters`, listing only the chapters overlapping the part. Each answer SHALL be added to `{part_summaries}` through `reduce part`, in order.

#### Scenario: Second part of three
- **WHEN** the second of 3 parts runs from 45:00 to 1:30:10
- **THEN** its user message has part `2` of `3`, span `45:00–1:30:10` and only the chapters overlapping that span

#### Scenario: Chapter starting on the part's last line
- **WHEN** a part's last line starts at 29:00 and a chapter starts at 29:00
- **THEN** that part lists the chapter

#### Scenario: Reduce
- **WHEN** all 3 parts are answered
- **THEN** the reduce user message holds the 3 answers in order, each filled through `reduce part`

### Requirement: Answer Cleaned
Everything up to and including the last `</think>` in an answer SHALL be dropped, and the result trimmed, for part answers and the final one, because some models write their reasoning into the answer even when told not to.

#### Scenario: Reasoning in the answer
- **WHEN** Ollama answers `<think>hmm</think>\n\nThree games.`
- **THEN** the answer used is `Three games.`

### Requirement: Incomplete Answers Rejected
A call SHALL be rejected when its answer is empty once cleaned, when Ollama stopped at the context limit, when Ollama refuses the prompt as too long, or when it runs past the 30-minute timeout. A rejected call SHALL fail the whole generation, storing nothing.

#### Scenario: Stopped at the output cap
- **WHEN** Ollama reports it stopped because of the length limit
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
When Ollama can't be reached before generating, or the saved model isn't installed on it, the generation SHALL stop with a logged warning, changing nothing, and the task SHALL complete, so a later sync tries again.

#### Scenario: Ollama down
- **WHEN** a summary task runs while nothing answers at the Ollama URL
- **THEN** the video stays Missing and the task completes

#### Scenario: Model not installed
- **WHEN** the saved model isn't among those Ollama lists
- **THEN** the video stays Missing and the task completes

### Requirement: Failed Summaries Retried
A generation that fails SHALL fail its task, so the task queue retries it with its usual backoff. On the last attempt, a rejected call or an unreadable transcript SHALL set the summary state to Unavailable, while a server fault (Ollama unreachable, or answering an error other than 400) SHALL leave the video Missing for a later sync.

#### Scenario: Failure before the last attempt
- **WHEN** a generation fails on an attempt that isn't the last
- **THEN** the task fails and the video stays Missing

#### Scenario: Rejected on the last attempt
- **WHEN** Ollama stops at the output cap on the last attempt
- **THEN** the task fails and the video's summary state is Unavailable

#### Scenario: Server fault on the last attempt
- **WHEN** Ollama answers 500 on the last attempt
- **THEN** the task fails and the video stays Missing

#### Scenario: Subtitle file missing
- **WHEN** a video's recorded subtitle file is no longer on disk on the last attempt
- **THEN** Ollama is not asked to summarize and the video's summary state is Unavailable

### Requirement: Unavailable Summaries Reset
Saving a change to the Ollama URL or to any summary setting (model, context length, temperature, thinking, prompt) SHALL put every video with summary state Unavailable back to Missing, because the new settings may succeed. Saving the same values SHALL change nothing.

#### Scenario: Context length raised
- **WHEN** a video is Unavailable and the context length is changed
- **THEN** the video is Missing and a later sync schedules its summary

#### Scenario: Unrelated change
- **WHEN** a video is Unavailable and a preference unrelated to summaries is changed
- **THEN** the video stays Unavailable

### Requirement: Summary Deleted With Its Video
A video's summary SHALL be deleted whenever the video's record is deleted, including when its whole playlist or channel is deleted.

#### Scenario: Video removed
- **WHEN** a video with a stored summary is removed from its playlist or channel and its record deleted
- **THEN** its summary is deleted too

#### Scenario: Playlist deleted
- **WHEN** a tracked playlist whose videos have stored summaries is deleted
- **THEN** those summaries are deleted too

### Requirement: Summary Generation Logged
Generation SHALL be logged at info level from start to end: the start with the video and its length in seconds; each call with its full system and user messages, then its full answer, tokens and duration; and the end with the summary, total duration, video length and model. Skips SHALL be logged with their reason at info level, and failures at error level with the specific reason: the context, input tokens and room left when the context ran out, or Ollama's refusal.

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

### Requirement: Summary Visibility
A stored summary SHALL be shown only while AI is on, summaries are on and the source holding that copy of the video has `summaries_enabled` `true`. Otherwise the system SHALL treat the video as having no summary wherever it is shown. A hidden summary SHALL stay stored and SHALL show again, without being regenerated, once all three conditions hold.

#### Scenario: AI turned off
- **WHEN** a video has a stored summary and AI is off
- **THEN** its summary is not shown and stays stored

#### Scenario: Summaries turned off
- **WHEN** a video has a stored summary, AI is on and summaries are off
- **THEN** its summary is not shown and stays stored

#### Scenario: Source opted out
- **WHEN** a video has a stored summary, AI and summaries are on, and its channel has `summaries_enabled` `false`
- **THEN** its summary is not shown and stays stored

#### Scenario: Source opted back in
- **WHEN** a channel with a hidden summary turns `summaries_enabled` back on
- **THEN** the summary is shown again and Ollama is not called

#### Scenario: Same video in two sources
- **WHEN** a YouTube video is tracked by a channel with `summaries_enabled` `true` and a playlist with it `false`, and both copies have a stored summary
- **THEN** the channel's copy shows its summary and the playlist's copy shows none
