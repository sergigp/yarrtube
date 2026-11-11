# Spec Delta

## ADDED Requirements

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
