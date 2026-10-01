## ADDED Requirements

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
