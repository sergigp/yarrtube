# Spec Delta

## MODIFIED Requirements

### Requirement: Ollama Unavailable
When Ollama can't be connected to before or during generation, or the saved model isn't installed on it, the generation SHALL stop with a logged warning, store nothing, leave the summary's status and failed attempts unchanged and re-queue it 6 hours later (see "Waiting For Ollama" in `video-retries`), without failing the task.

#### Scenario: Ollama down
- **WHEN** a summary task runs while nothing answers at the Ollama URL
- **THEN** Ollama is not asked to summarize, the summary keeps its status and failed attempts, and it is re-queued 6 hours later

#### Scenario: Model not installed
- **WHEN** the saved model isn't among those Ollama lists
- **THEN** Ollama is not asked to summarize, the summary keeps its status and failed attempts, and it is re-queued 6 hours later

#### Scenario: Ollama goes down between parts
- **WHEN** a transcript in 3 parts is summarized and Ollama can't be connected to for the second part's call
- **THEN** no summary is stored, the summary keeps its status and failed attempts, and it is re-queued 6 hours later

### Requirement: Failed Summaries Follow The Retry Policy
A rejected call, an unreadable transcript, or a server fault during generation (Ollama answering an error other than 400, or failing after accepting the connection) SHALL make the summary Retrying with one more failed attempt and re-queue it under the `video-retries` policy, without failing the task. After the 11th consecutive failed attempt the summary SHALL become Failed. A generated summary SHALL make it Done and clear the failed attempts.

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
