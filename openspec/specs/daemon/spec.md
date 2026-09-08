# daemon Specification

## Purpose

Defines the behavior of yarrtube's long-running daemon process: what it does on startup and while running. It is the placeholder that a future task scheduler will be built into, so it exists today mainly to prove its dependencies work and to stay alive.

## Requirements

### Requirement: Startup Self-Update
The system SHALL attempt to update its yt-dlp binary once each time the daemon starts, using the same behavior as the `update-ytdlp` task.

#### Scenario: Successful startup update
- **WHEN** the daemon starts and the yt-dlp update succeeds
- **THEN** the daemon logs the update result and continues starting up

#### Scenario: Startup update fails
- **WHEN** the daemon starts and the yt-dlp update fails (for example, due to no network access)
- **THEN** the daemon logs the failure and continues starting up rather than exiting, so a temporary network issue does not prevent the container from running

### Requirement: Database Connectivity Check
The system SHALL, on startup, open (creating if it does not exist) a local database file and execute a trivial query against it to confirm the database is usable.

#### Scenario: Database usable
- **WHEN** the daemon starts and the database file can be opened and queried
- **THEN** the daemon logs that the database check succeeded

#### Scenario: Database unusable
- **WHEN** the daemon starts and the database file cannot be opened or queried
- **THEN** the daemon logs the failure clearly, identifying it as a database problem

### Requirement: Heartbeat Logging
The system SHALL log a heartbeat message on a recurring interval for as long as the daemon is running, to provide a visible signal that the process is alive.

#### Scenario: Daemon running past one interval
- **WHEN** the daemon has been running longer than one heartbeat interval
- **THEN** at least one heartbeat message has been logged

### Requirement: HTTP Server Availability
The system SHALL start an HTTP server during daemon startup and keep it accepting connections for as long as the daemon process runs.

#### Scenario: Server available for the process lifetime
- **WHEN** the daemon is running
- **THEN** its HTTP server accepts connections until the daemon process is stopped

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
