# Spec Delta

## ADDED Requirements

### Requirement: Run Tasks Now
The system SHALL let a client bring pending tasks forward (`POST /api/tasks/run-now` with `{"ids": [...]}`, answering 204). Each listed task that is pending SHALL get the current time as its run time, keeping its type, payload, retry count and last error, and then runs under the usual lane rules. Listed ids of running or unknown tasks SHALL be ignored. A missing or empty `ids` list SHALL answer 400.

#### Scenario: Bring a reconciliation forward
- **WHEN** a client asks to run now a channel reconciliation due in 17 minutes
- **THEN** the server answers 204 and the reconciliation is eligible to run at once

#### Scenario: Several tasks
- **WHEN** a client asks to run now 16 pending reconciliations
- **THEN** all 16 are eligible at once, and the light lane still runs them one at a time in order of when they became eligible

#### Scenario: Running or unknown ids
- **WHEN** a client asks to run now one pending task, one running task and an id that does not exist
- **THEN** the server answers 204, the pending task is eligible at once, and the running task is unchanged

#### Scenario: Retry state kept
- **WHEN** a client asks to run now a pending task with 2 retries and a last error
- **THEN** the task keeps 2 retries and its last error, and only its run time changes

#### Scenario: No ids
- **WHEN** a client asks to run now with an empty `ids` list
- **THEN** the server answers 400 and no task changes
