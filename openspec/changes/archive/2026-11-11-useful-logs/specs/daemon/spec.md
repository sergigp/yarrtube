# Spec Delta

## MODIFIED Requirements

### Requirement: Heartbeat Logging
The system SHALL log a heartbeat message every 5 minutes for as long as the daemon is running, to provide a visible signal that the process is alive. The heartbeat SHALL carry the number of pending tasks per lane, the number of running tasks, the age of the oldest eligible pending task, the number of pending events, the number of dead-lettered tasks and events, and the number of videos pending, retrying and failed for each of download, summary and categorization. A heartbeat whose counts cannot be read SHALL still be logged, with the read failure.

#### Scenario: Daemon running past one interval
- **WHEN** the daemon has been running longer than one heartbeat interval
- **THEN** at least one heartbeat message has been logged

#### Scenario: Heartbeat reports the queue and the library
- **WHEN** a heartbeat is logged while two download tasks and one sync task are pending, one task is running, one event is pending, one task is dead-lettered and one video's download is retrying
- **THEN** the heartbeat carries those counts, the age of the oldest eligible pending task, and zero for every other count

#### Scenario: Counts cannot be read
- **WHEN** reading the heartbeat's counts fails, for example because the database is unavailable
- **THEN** a heartbeat is still logged, at warn level, with the error
