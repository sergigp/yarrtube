## MODIFIED Requirements

### Requirement: Retry With Fixed Delay Then Give Up
The system SHALL track a retry counter per task. On failure, the counter SHALL increment and the task SHALL become eligible again after a delay that grows with the number of prior attempts, starting from a configurable base delay, rather than a fixed delay. After 5 failed attempts, the system SHALL log the failure, move the task to the dead-letter table, and remove it from the tasks table so it is not attempted again.

#### Scenario: A task attempt fails
- **WHEN** a task's handler raises an error
- **THEN** the task's retry counter increments and it becomes eligible again after a delay that is longer than the delay used before its previous attempt

#### Scenario: Fifth consecutive failure
- **WHEN** a task's retry counter reaches 5 failed attempts
- **THEN** the system logs the failure, records the task's id, type, payload, final error, attempt count, and timestamps in the dead-letter table, and deletes the task from the tasks table

#### Scenario: Base retry delay not configured
- **WHEN** no base retry delay has been explicitly configured
- **THEN** the system uses its default base retry delay, applied uniformly to every task type

#### Scenario: Base retry delay configured
- **WHEN** a base retry delay has been explicitly configured
- **THEN** the system uses that value, applied uniformly to every task type, in place of the default
