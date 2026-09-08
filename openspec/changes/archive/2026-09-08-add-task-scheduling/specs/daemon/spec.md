## ADDED Requirements

### Requirement: Startup Task Recovery
The system SHALL, during startup and before beginning normal task execution, recover any task left in a running state by a previous, interrupted run of the process.

#### Scenario: Restart after an unclean shutdown
- **WHEN** the daemon starts and the tasks table contains a task in a running state
- **THEN** the daemon recovers it (per the task-scheduling capability's crash recovery behavior) before task execution begins

### Requirement: Domain Event Consumption
The system SHALL start a domain event consumer during daemon startup and keep it polling for pending events for as long as the daemon process runs.

#### Scenario: Consumer available for the process lifetime
- **WHEN** the daemon is running
- **THEN** its domain event consumer continues polling for pending events until the daemon process is stopped

### Requirement: Task Execution
The system SHALL start a task executor during daemon startup and keep it polling for eligible tasks for as long as the daemon process runs.

#### Scenario: Executor available for the process lifetime
- **WHEN** the daemon is running
- **THEN** its task executor continues polling for eligible tasks until the daemon process is stopped
