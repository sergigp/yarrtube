# task-scheduling Specification

## Purpose

Provides a persisted, restart-safe mechanism for scheduling work to run at or after a specific time and executing it through exactly one handler per task, so scheduled and recurring work survives process restarts.

## Requirements

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
The system SHALL track a retry counter per task. On failure, the counter SHALL increment and the task SHALL become eligible again after a short fixed delay. After 5 failed attempts, the system SHALL log the failure, move the task to the dead-letter table, and remove it from the tasks table so it is not attempted again.

#### Scenario: A task attempt fails
- **WHEN** a task's handler raises an error
- **THEN** the task's retry counter increments and it becomes eligible again after a fixed delay

#### Scenario: Fifth consecutive failure
- **WHEN** a task's retry counter reaches 5 failed attempts
- **THEN** the system logs the failure, records the task's id, type, payload, final error, attempt count, and timestamps in the dead-letter table, and deletes the task from the tasks table

### Requirement: Dead-Letter Record for Permanently Failed Tasks
The system SHALL persist a durable record of every task that exhausts its retries, so a permanently failed task remains inspectable after it leaves the tasks table.

#### Scenario: Task moved to dead letter
- **WHEN** a task reaches its 5th failed attempt
- **THEN** a record containing the task's original id, type, payload, final error message, number of attempts, and timestamps is inserted into the dead-letter table

### Requirement: Tasks Table Behaves as a Queue
The system SHALL remove a task from the tasks table as soon as it reaches a terminal state, so the table only ever contains tasks that are pending, scheduled for retry, or currently running.

#### Scenario: Task completes successfully
- **WHEN** a task's handler returns successfully
- **THEN** the task is deleted from the tasks table rather than being retained with a completed status

#### Scenario: Task exhausts retries
- **WHEN** a task reaches its 5th failed attempt, including one recovered from a crash, and is moved to the dead-letter table
- **THEN** it is also deleted from the tasks table

### Requirement: Crash Recovery
The system SHALL, on startup and before resuming normal task execution, treat any task still marked as running from a previous, interrupted process as a failed attempt.

#### Scenario: Daemon restarts with a task stuck running
- **WHEN** the daemon starts and finds a task in a running state left over from a previous run
- **THEN** it increments that task's retry counter and applies the same retry-then-give-up handling as any other failed attempt, before task execution resumes
