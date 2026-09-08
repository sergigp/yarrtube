## Purpose

Provides a persisted, restart-safe mechanism for scheduling work to run at or after a specific time and executing it through exactly one handler per task, so scheduled and recurring work survives process restarts.

## ADDED Requirements

### Requirement: Scheduling a Task
The system SHALL allow a task to be scheduled with a type, a payload, and a time at or after which it becomes eligible to run.

#### Scenario: Task scheduled for now
- **WHEN** a task is scheduled with a run time at or before the current time
- **THEN** it is immediately eligible to run

#### Scenario: Task scheduled for a future time
- **WHEN** a task is scheduled with a run time in the future
- **THEN** it does not become eligible to run until that time has passed

### Requirement: Single Execution Per Attempt
The system SHALL poll for eligible tasks in the background and dispatch each one to exactly one handler based on its type.

#### Scenario: Eligible task is dispatched
- **WHEN** a task's run time has passed and it has not yet succeeded
- **THEN** the system dispatches it to the handler registered for its type

### Requirement: Retry With Fixed Delay Then Give Up
The system SHALL track a retry counter per task. On failure, the counter SHALL increment and the task SHALL become eligible again after a short fixed delay. After 5 failed attempts, the task SHALL be marked permanently failed, logged, and not attempted again.

#### Scenario: A task attempt fails
- **WHEN** a task's handler raises an error
- **THEN** the task's retry counter increments and it becomes eligible again after a fixed delay

#### Scenario: Fifth consecutive failure
- **WHEN** a task's retry counter reaches 5 failed attempts
- **THEN** the system logs the failure, marks the task permanently failed, and does not attempt it again

### Requirement: Crash Recovery
The system SHALL, on startup and before resuming normal task execution, treat any task still marked as running from a previous, interrupted process as a failed attempt.

#### Scenario: Daemon restarts with a task stuck running
- **WHEN** the daemon starts and finds a task in a running state left over from a previous run
- **THEN** it increments that task's retry counter and applies the same retry-then-give-up handling as any other failed attempt, before task execution resumes
