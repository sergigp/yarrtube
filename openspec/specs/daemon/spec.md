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

### Requirement: Resilient Database Access Under Concurrent Load
The daemon SHALL serialize all of its writes through a single writer so that concurrent write operations complete in turn rather than failing with a database-locked error, and SHALL run reads on a separate connection so that an in-progress write burst does not block them. A burst of writes triggered by one operation (for example, tracking a playlist with many videos, which schedules a download and a thumbnail fetch per video) SHALL NOT cause another concurrent database operation in the same process to fail, and SHALL NOT prevent a concurrent read (such as the web UI listing videos) from completing.

This requirement is about the process not contending with itself. It does not extend to a separate process or external tool that opens the same database file; against those the daemon MAY still wait and, after a bounded time, surface a database-locked error.

#### Scenario: Concurrent writes during a large playlist ingest
- **WHEN** many database writes are issued concurrently within the daemon while it is also polling and claiming scheduled tasks
- **THEN** every write and every task claim completes without a database-locked error, each proceeding in turn

#### Scenario: Task claiming is not starved by a write burst
- **WHEN** the task executor claims an eligible task while a burst of other database writes is in progress
- **THEN** the claim succeeds rather than returning a database-locked error

#### Scenario: Reads are not blocked by a write burst
- **WHEN** a read is issued while a write is in progress within the daemon
- **THEN** the read completes without waiting for the write, returning the latest committed data

### Requirement: Startup Default Storage Directory Creation
The system SHALL, on startup, ensure the default parent storage directories (`playlists` and `channels`) exist under the configured videos root, creating any that are missing. A directory that already exists SHALL be left untouched, with its contents unchanged.

This SHALL happen at daemon startup rather than when the container image is built. The videos root is a mount point, and a host directory mounted there at container start replaces the path entirely, masking anything the image created under it — so directories created at build time would never be visible to a running container.

Failure to create a directory SHALL NOT prevent the daemon from starting: the daemon SHALL log the failure and continue, consistent with its other startup steps, so that a read-only or not-yet-ready mount does not take the service down.

#### Scenario: Default directories missing
- **WHEN** the daemon starts and the videos root contains neither default parent directory
- **THEN** the daemon creates both of them and logs that it did so

#### Scenario: Default directories already present
- **WHEN** the daemon starts and the default parent directories already exist under the videos root
- **THEN** the daemon leaves them and their contents unchanged

#### Scenario: Default directory creation fails
- **WHEN** the daemon starts and a default parent directory cannot be created, for example because the videos root is mounted read-only
- **THEN** the daemon logs the failure clearly, identifying it as a storage directory problem, and continues starting up rather than exiting

#### Scenario: Directories are visible to a browsing client
- **WHEN** the daemon has started against an empty mounted videos root
- **THEN** a client listing the videos root sees the default parent directories

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
