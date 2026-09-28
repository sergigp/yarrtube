## MODIFIED Requirements

### Requirement: Single Execution Per Attempt
The system SHALL poll for eligible tasks in the background and dispatch each one to exactly one handler based on its type. When several tasks are eligible in the same poll, the system SHALL dispatch them in ascending order of the time they became eligible to run, breaking ties by the order in which they were scheduled.

#### Scenario: Eligible task is dispatched
- **WHEN** a task's run time has passed and it has not yet succeeded
- **THEN** the system dispatches it to the handler registered for its type

#### Scenario: Earlier-due task runs first regardless of scheduling order
- **WHEN** two tasks are eligible in the same poll and the one scheduled later has an earlier run time
- **THEN** the system dispatches the task with the earlier run time first

#### Scenario: Tasks with the same run time
- **WHEN** two tasks are eligible in the same poll with identical run times
- **THEN** the system dispatches the one that was scheduled first before the other
