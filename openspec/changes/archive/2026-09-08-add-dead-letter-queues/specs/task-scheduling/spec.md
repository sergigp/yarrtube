## MODIFIED Requirements

### Requirement: Retry With Fixed Delay Then Give Up
The system SHALL track a retry counter per task. On failure, the counter SHALL increment and the task SHALL become eligible again after a short fixed delay. After 5 failed attempts, the system SHALL log the failure, move the task to the dead-letter table, and remove it from the tasks table so it is not attempted again.

#### Scenario: A task attempt fails
- **WHEN** a task's handler raises an error
- **THEN** the task's retry counter increments and it becomes eligible again after a fixed delay

#### Scenario: Fifth consecutive failure
- **WHEN** a task's retry counter reaches 5 failed attempts
- **THEN** the system logs the failure, records the task's id, type, payload, final error, attempt count, and timestamps in the dead-letter table, and deletes the task from the tasks table

## ADDED Requirements

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
