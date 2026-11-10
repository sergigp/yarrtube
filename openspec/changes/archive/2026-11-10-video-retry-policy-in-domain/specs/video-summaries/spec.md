## RENAMED Requirements

- FROM: `### Requirement: Unavailable Summaries Reset`
- TO: `### Requirement: Failed Summaries Reset`

## MODIFIED Requirements

### Requirement: Summary State
Every stored video SHALL have a summary status from the `video-retries` vocabulary: Pending (formerly Missing), Retrying, Done (formerly Generated) or Failed (formerly Unavailable), stored as `PENDING`, `RETRYING`, `DONE` and `FAILED`. Videos stored before this state existed SHALL be Pending. Done is final. Failed lasts until the summary settings change or the user retries the video.

#### Scenario: Existing videos after upgrade
- **WHEN** the daemon starts on a database with videos stored before summaries existed
- **THEN** every one of them has summary status Pending

#### Scenario: Stored states upgraded
- **WHEN** the system starts on a database with summary states `MISSING`, `GENERATED` and `UNAVAILABLE`
- **THEN** they become `PENDING`, `DONE` and `FAILED`

### Requirement: Failed Summaries Reset
Saving a change to the Ollama URL or to any summary setting (model, context length, temperature, thinking, prompt) SHALL put every video whose summary is Failed back to Pending with no failed attempts, because the new settings may succeed. Saving the same values SHALL change nothing.

#### Scenario: Context length raised
- **WHEN** a video's summary is Failed and the context length is changed
- **THEN** its summary is Pending and a later sync schedules its summary

#### Scenario: Unrelated change
- **WHEN** a video's summary is Failed and a preference unrelated to summaries is changed
- **THEN** its summary stays Failed

### Requirement: Ollama Unavailable
When Ollama can't be reached before generating, or the saved model isn't installed on it, the generation SHALL stop with a logged warning, make the summary Retrying with one more failed attempt and re-queue it under the `video-retries` policy, without failing the task.

#### Scenario: Ollama down
- **WHEN** a summary task runs while nothing answers at the Ollama URL
- **THEN** the summary is Retrying with one failed attempt and is re-queued for its next attempt

#### Scenario: Model not installed
- **WHEN** the saved model isn't among those Ollama lists
- **THEN** the summary is Retrying with one failed attempt and is re-queued for its next attempt

## ADDED Requirements

### Requirement: Failed Summaries Follow The Retry Policy
A rejected call, an unreadable transcript, or a server fault during generation (Ollama unreachable, or answering an error other than 400) SHALL make the summary Retrying with one more failed attempt and re-queue it under the `video-retries` policy, without failing the task. After the 11th consecutive failed attempt the summary SHALL become Failed. A generated summary SHALL make it Done and clear the failed attempts.

#### Scenario: Rejected
- **WHEN** Ollama stops at the output cap on a summary's first attempt
- **THEN** the summary is Retrying with one failed attempt and is re-queued 10 minutes later

#### Scenario: Server fault
- **WHEN** Ollama answers 500 during generation
- **THEN** the summary is Retrying with one more failed attempt and the task does not fail

#### Scenario: Eleventh failure
- **WHEN** a summary with 10 consecutive failed attempts is rejected again
- **THEN** the summary is Failed and the task completes

#### Scenario: Subtitle file missing
- **WHEN** a video's recorded subtitle file is no longer on disk
- **THEN** Ollama is not asked to summarize, and the summary is Retrying with one more failed attempt

## REMOVED Requirements

### Requirement: Failed Summaries Retried
**Reason**: Replaced by "Failed Summaries Follow The Retry Policy": failures back off and give up after 11 attempts on the video, instead of on the task queue's 5th attempt.
**Migration**: Stored `UNAVAILABLE` summaries become `FAILED`, kept until the summary settings change or the user retries the video.
