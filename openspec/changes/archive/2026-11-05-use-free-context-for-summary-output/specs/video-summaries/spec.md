## MODIFIED Requirements

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
