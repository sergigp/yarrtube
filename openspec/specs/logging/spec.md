# logging Specification

## Purpose

Defines how the daemon reports its own operational activity: at what
severity levels, how that verbosity is controlled at startup, and what
happens to that output in the automated test environment.

## Requirements

### Requirement: Leveled Log Output
The system SHALL categorize every operational message it emits (startup
checks, background sync/task/event activity, and fatal errors) at one of the
standard severity levels — error, warn, info, or debug — reflecting the
message's actual severity, and SHALL attach the relevant identifying values
(e.g. a playlist or task id) to the message as structured, queryable fields
rather than only interpolating them into free text.

#### Scenario: Routine activity is informational
- **WHEN** the daemon completes a routine operation (e.g. a successful
  startup check, a scheduled sync, a dispatched task)
- **THEN** the message is emitted at info level or below, not warn or error

#### Scenario: Failure is emitted at error level
- **WHEN** an operation fails in a way that prevents it from completing (e.g.
  a startup check fails, a background poll fails, a task or event is marked
  permanently failed)
- **THEN** the message is emitted at error level

#### Scenario: Identifying values are attached as fields
- **WHEN** a log message refers to a specific entity (a playlist, video,
  task, or event id)
- **THEN** that identifier is attached to the message as a named field, not
  only embedded in the message text

### Requirement: Configurable Verbosity at Startup
The system SHALL allow an operator to control which severity levels are
emitted by setting an environment variable when starting the daemon, without
requiring a rebuild, following the prevailing convention for the language
runtime the system is implemented in. The system SHALL emit info level and
above when the environment variable is not set.

#### Scenario: Operator raises verbosity
- **WHEN** the daemon is started with the log-level environment variable set
  to a more verbose level than the default (e.g. debug)
- **THEN** messages at that level and above are emitted

#### Scenario: Operator lowers verbosity
- **WHEN** the daemon is started with the log-level environment variable set
  to a less verbose level than the default (e.g. error only)
- **THEN** only messages at that level and above are emitted, and lower-
  severity messages (e.g. routine info-level activity) are suppressed

#### Scenario: No configuration provided
- **WHEN** the daemon is started without the log-level environment variable
  set
- **THEN** info-level and above messages are emitted, matching today's
  default visible output

### Requirement: No Log Output in Automated Tests
The system SHALL NOT emit log output during automated test runs, so test
output stays limited to test framework reporting (pass/fail, panics,
assertion failures).

#### Scenario: Test suite runs
- **WHEN** the automated test suite executes any test that exercises code
  paths which log
- **THEN** no log lines appear in the test run's output

### Requirement: External Process Diagnostics Reported Through Structured Logs
The system SHALL NOT write the raw diagnostic output (stderr) of the external
download tool (`yt-dlp`) directly to the daemon's log output. When that output
matters for an operation's outcome, the system SHALL report it as part of a
leveled log message carrying the identifying values of the operation (e.g. the
video or channel concerned) as structured fields, consistent with the
"Leveled Log Output" requirement.

#### Scenario: Thumbnail fetch fails with a tool error
- **WHEN** a thumbnail fetch runs and `yt-dlp` exits with an error such as "Video unavailable"
- **THEN** no raw `yt-dlp` stderr line appears in the daemon's log output, and the failure is emitted as one leveled message with the video's id attached as a field and the tool's reason included

#### Scenario: Channel listing fails with a tool error
- **WHEN** listing a tracked channel's videos makes `yt-dlp` exit with an error
- **THEN** no raw `yt-dlp` stderr line appears in the daemon's log output, and the failure is emitted as one leveled message with the channel attached as a field and the tool's reason included

#### Scenario: Tool succeeds
- **WHEN** a thumbnail fetch or channel listing completes successfully
- **THEN** no `yt-dlp` stderr output appears in the daemon's log output

### Requirement: YouTube API Key Never Logged
The system SHALL NOT include the YouTube Data API key in any log output or in
any error recorded for a task, including errors raised when a YouTube API
request fails before receiving a response (for example a DNS, connection or
timeout failure) and errors carrying a YouTube API error response.

#### Scenario: YouTube API request fails to connect
- **WHEN** a YouTube Data API request fails before a response arrives, for example because the host name cannot be resolved
- **THEN** the logged error and the task's recorded error describe the failure without containing the API key

#### Scenario: YouTube API returns an error response
- **WHEN** a YouTube Data API request receives a non-success response
- **THEN** the logged error and the task's recorded error contain the status and response body without containing the API key

### Requirement: One Info Line Per Finished Task
The system SHALL log one info line when a task attempt ends, carrying the task's id, type, the video, channel or playlist id from its payload when it has one, the outcome, how long the attempt ran and how long the task waited past its scheduled run time.

#### Scenario: Task completes
- **WHEN** a task's handler completes it
- **THEN** one info line reports the task id, type, payload id, outcome `done`, run duration and queue wait

#### Scenario: Task asks to run again later
- **WHEN** a task's handler asks for it to run again at a later time
- **THEN** one info line reports outcome `requeued` with the next run time and reason, plus the same identifiers and timings

#### Scenario: Task attempt fails
- **WHEN** a task attempt fails and is retried or dead-lettered
- **THEN** the existing warn (retry, with next run time and attempt number) or error (dead-lettered) line carries the error and the same identifiers and timings, and no separate end line is logged for that attempt

### Requirement: Task That Changed Nothing Is Reported as Unchanged
The system SHALL let a task report that its run changed nothing, and SHALL then complete it like a done task, its end line reporting outcome `unchanged`. A `sync_video` task whose video needed nothing SHALL report this.

#### Scenario: Video sync with nothing to do
- **WHEN** a `sync_video` task runs for a video that has no step due, no broken file, no missing metadata and no legacy location
- **THEN** the task is completed and removed, its end line reports outcome `unchanged`, and no `synced video` line is logged

#### Scenario: Video sync that does something
- **WHEN** a `sync_video` task schedules a due step, resets the video for redownload, repairs its metadata or relocates it
- **THEN** one info line names the video and what the sync did, and the task's end line reports outcome `done`

### Requirement: One Info Line Per Finished Event
The system SHALL log one info line when an event's dispatch ends, carrying the event's id, type, payload, subscriber count, outcome and dispatch duration.

#### Scenario: Event handled by every subscriber
- **WHEN** every subscriber of an event handles it
- **THEN** one info line reports the event id, type, payload, subscriber count, outcome `done` and duration

#### Scenario: Event dispatch fails
- **WHEN** a subscriber fails and the event is retried or dead-lettered
- **THEN** the existing warn or error line carries the error and the same identifiers and duration, and no separate end line is logged for that dispatch

### Requirement: Reconcile Pass Summary
The system SHALL end every channel and playlist reconcile pass that completes with one info line carrying the source's id, the number of videos listed on YouTube (absent when the listing failed), added, removed, video syncs requested and orphan files deleted, the listing duration and the pass's total duration.

#### Scenario: Pass with no changes
- **WHEN** a reconcile pass finds the source's stored videos already match YouTube
- **THEN** one info line reports the counts, with zero added and removed, and the durations

#### Scenario: Pass whose listing fails
- **WHEN** a recurring reconcile pass cannot list the source's videos
- **THEN** the failure is logged as today and the summary line still ends the pass, with no listed count

### Requirement: Categorization Run Summary
The system SHALL log one info line per categorization run of a video, carrying the video's id and the number of categories asked about, answered yes, answered no, and rejected. Each call to the model SHALL be logged with whether thinking was requested and whether the reply stopped at the output limit.

#### Scenario: Run with yes and no verdicts
- **WHEN** a categorization run gets a yes for one category and a no for two others
- **THEN** one info line reports 3 asked, 1 yes, 2 no, 0 rejected

#### Scenario: Model call logged
- **WHEN** the categorizer or the summarizer receives a reply from the model
- **THEN** the line logging the call carries `thinking` and `stopped_at_limit`
