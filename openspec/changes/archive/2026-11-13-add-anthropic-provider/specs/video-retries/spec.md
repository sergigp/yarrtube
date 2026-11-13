## MODIFIED Requirements

### Requirement: What Counts As A Failed Attempt
A failed attempt SHALL be: for the download, any failure that is not permanently unavailable (see `video-download`), including `yt-dlp` failing to run and the files failing to be placed; for the thumbnail, a fetch that obtains no thumbnail; for subtitles, chapters and the quality check, a failed or inconclusive check (see `late-video-data`); for the summary and the categorization, a rejected call, an unusable answer, an unreadable transcript, or the AI provider answering a server error (for Anthropic, a server fault as `llm-connection` defines it: 429, 5xx or 529). A check that conclusively finds data not on YouTube yet, a step skipped because a setting turns it off, the AI provider being unavailable (see "Waiting For Ollama"), and a database error SHALL NOT count as failed attempts.

#### Scenario: Ollama down
- **WHEN** a summary runs while nothing answers at the Ollama URL
- **THEN** the summary keeps its status and failed attempts, and the summary is re-queued 6 hours later

#### Scenario: Ollama answers a server error
- **WHEN** a summary runs and Ollama answers 500 to its call
- **THEN** the summary is Retrying with one more failed attempt and its next attempt is scheduled

#### Scenario: Anthropic overloaded
- **WHEN** the provider is Anthropic and a summary's call is answered 529
- **THEN** the summary is Retrying with one more failed attempt and its next attempt is scheduled

#### Scenario: AI turned off
- **WHEN** a summary runs while AI is off
- **THEN** the summary stays Pending with no failed attempt and nothing is scheduled

#### Scenario: Captions not on YouTube yet
- **WHEN** a conclusive subtitle check finds no captions for a video published 2 days ago
- **THEN** its subtitles stay Pending with no failed attempt

#### Scenario: Database unavailable
- **WHEN** a step cannot read or store the video because the database fails
- **THEN** no failed attempt is counted on the video, and the task fails for the task queue to retry and dead-letter it

### Requirement: Waiting For Ollama
The AI provider SHALL be unavailable to a summary or categorization run when: with Ollama, it can't be connected to (refused, unknown host, or no connection within 5 seconds), before or during the run, or the saved model isn't among those it lists before the run; with Anthropic, no key is set, the saved model isn't among those it lists before the run, or a call finds it unavailable (see `llm-connection`). The run SHALL then stop, leave the step's status and failed attempts unchanged, and re-queue the step's task 6 hours later, with the reason as its last error, without failing the task. There SHALL be no limit on how many times a step waits.

#### Scenario: Pending summary waits
- **WHEN** a Pending summary runs at 10:00 while nothing answers at the Ollama URL
- **THEN** the summary is Pending with no failed attempts and its task is pending to run at 16:00

#### Scenario: Retrying step keeps its counter
- **WHEN** a summary Retrying with 4 failed attempts, last failed at 08:00 the day before, runs at 10:00 while Ollama can't be connected to
- **THEN** the summary is Retrying with 4 failed attempts, last failed at 08:00 the day before, and its task is pending to run at 16:00

#### Scenario: Last attempt not spent
- **WHEN** a categorization Retrying with 10 failed attempts runs while Ollama can't be connected to
- **THEN** it is Retrying with 10 failed attempts, no `failed` verdict is written, and it is re-queued 6 hours later

#### Scenario: Model not installed
- **WHEN** a summary runs and the saved model isn't among those Ollama lists
- **THEN** the summary keeps its status and failed attempts and is re-queued 6 hours later

#### Scenario: Slow answer still counts
- **WHEN** Ollama accepts the connection for a summary call but doesn't answer within the call's timeout
- **THEN** the call is rejected and the summary is Retrying with one more failed attempt

#### Scenario: Ollama back
- **WHEN** a summary that waited for Ollama runs again 6 hours later and Ollama answers usably
- **THEN** the summary is Done with no failed attempts

#### Scenario: Out of credit keeps every attempt
- **WHEN** the provider is Anthropic, 50 summaries are queued and every call answers that the credit balance is too low
- **THEN** every summary keeps its status and failed attempts and is re-queued 6 hours later, with the reason as its last error
