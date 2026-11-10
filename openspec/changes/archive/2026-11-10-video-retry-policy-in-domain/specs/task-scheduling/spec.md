## ADDED Requirements

### Requirement: Handler Re-queues Its Task
A task handler SHALL be able to complete a task by asking for it to run again at a later time, with a reason. The system SHALL then keep the task pending with that run time, that reason as its last error and its retry counter back at 0, instead of deleting it. Re-queueing SHALL NOT count as a failed attempt and SHALL never dead-letter the task.

#### Scenario: Download re-queued after a failed attempt
- **WHEN** a download handler completes asking to run again at 10:10 because "HTTP Error 503"
- **THEN** the task stays pending with run time 10:10, last error "HTTP Error 503" and 0 retries

#### Scenario: Re-queued task that had failed before
- **WHEN** a task that failed twice through handler errors is then re-queued by its handler
- **THEN** its retry counter is 0, so a later handler error starts its backoff from the first delay

## MODIFIED Requirements

### Requirement: Retry With Fixed Delay Then Give Up
The system SHALL track a retry counter per task. When a task's handler raises an error (or panics), the counter SHALL increment and the task SHALL become eligible again after a delay that grows with the number of prior attempts, starting from a configurable base delay, rather than a fixed delay. After 5 failed attempts, the system SHALL log the failure, move the task to the dead-letter table, and remove it from the tasks table so it is not attempted again. A handler SHALL raise an error only for failures its domain does not handle itself (such as the database being unavailable); handlers are not told how many attempts remain.

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
