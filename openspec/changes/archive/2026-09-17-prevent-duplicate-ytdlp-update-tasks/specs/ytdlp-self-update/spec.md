## MODIFIED Requirements

### Requirement: Manual and Automatic Invocation
The system SHALL support running the yt-dlp update as an explicit ad hoc command, automatically once each time the daemon starts, and automatically on a recurring interval for as long as the daemon runs. Seeding the recurring interval at startup SHALL be idempotent: it SHALL NOT create an additional recurring chain if one is already scheduled and has not yet reached a terminal state.

#### Scenario: Manual invocation
- **WHEN** an operator runs `update-ytdlp` directly
- **THEN** it performs the update independent of whether the daemon is running

#### Scenario: Automatic invocation at daemon startup
- **WHEN** the daemon starts
- **THEN** it performs the same update behavior as running `update-ytdlp` manually

#### Scenario: Recurring automatic invocation while running
- **WHEN** the daemon has been running longer than the recurring update interval
- **THEN** the system has attempted the yt-dlp update again, independently of the startup attempt, and continues to do so on that interval for as long as the daemon runs

#### Scenario: Daemon restarts while a recurring update is already scheduled
- **WHEN** the daemon starts and a recurring yt-dlp update task from a previous run already exists in a pending or running state
- **THEN** the system does not schedule a second recurring chain, so exactly one recurring update continues on its existing schedule

#### Scenario: Daemon starts with no recurring update scheduled
- **WHEN** the daemon starts and no recurring yt-dlp update task exists in a pending or running state
- **THEN** the system schedules a new recurring update chain, as before
