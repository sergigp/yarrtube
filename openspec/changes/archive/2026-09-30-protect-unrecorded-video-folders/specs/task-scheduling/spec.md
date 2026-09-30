## MODIFIED Requirements

### Requirement: Crash Recovery
The system SHALL, on startup and before resuming normal task execution, treat any task still marked as running from a previous, interrupted process as a failed attempt. While running, the system SHALL also treat any task still marked as running that no attempt in the current process is running (for example, because recording its outcome failed) as a failed attempt, no later than its next check for eligible tasks, so such a task neither waits for a restart nor keeps blocking later tasks for its video (see "No Duplicate Per-Video Tasks").

#### Scenario: Daemon restarts with a task stuck running
- **WHEN** the daemon starts and finds a task in a running state left over from a previous run
- **THEN** it increments that task's retry counter and applies the same retry-then-give-up handling as any other failed attempt, before task execution resumes

#### Scenario: Task left running with no attempt in progress
- **WHEN** the daemon is running and a task is marked as running although no attempt of it is in progress, for example because its outcome could not be recorded
- **THEN** on its next check for eligible tasks the system increments that task's retry counter and applies the same retry-then-give-up handling as any other failed attempt, without waiting for a restart

#### Scenario: Later task for the same video after recovery
- **WHEN** a download task left running with no attempt in progress has been recovered, and a new download is then scheduled for the same video
- **THEN** the new request is treated like any other request for a video that already has a pending download
