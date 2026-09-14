## ADDED Requirements

### Requirement: Configured Path Is Authoritative For Invocation
The system SHALL invoke `yt-dlp` for every video download at the same configured path that `update-ytdlp` installs to, rather than resolving `yt-dlp` via a `PATH` lookup that may diverge from that location.

#### Scenario: Download invokes the configured binary
- **WHEN** a video download is attempted
- **THEN** the system runs the `yt-dlp` binary at the currently configured install path, not a separately `PATH`-resolved binary that may be a different file

#### Scenario: Configured path has no binary
- **WHEN** a video download is attempted and no binary exists at the configured path
- **THEN** the system reports that `yt-dlp` is unavailable, the same way it does today when the binary is missing

## MODIFIED Requirements

### Requirement: Manual and Automatic Invocation
The system SHALL support running the yt-dlp update as an explicit ad hoc command, automatically once each time the daemon starts, and automatically on a recurring interval for as long as the daemon runs.

#### Scenario: Manual invocation
- **WHEN** an operator runs `update-ytdlp` directly
- **THEN** it performs the update independent of whether the daemon is running

#### Scenario: Automatic invocation at daemon startup
- **WHEN** the daemon starts
- **THEN** it performs the same update behavior as running `update-ytdlp` manually

#### Scenario: Recurring automatic invocation while running
- **WHEN** the daemon has been running longer than the recurring update interval
- **THEN** the system has attempted the yt-dlp update again, independently of the startup attempt, and continues to do so on that interval for as long as the daemon runs

### Requirement: Failure Isolation
The system SHALL leave the existing yt-dlp binary usable if `update-ytdlp` fails partway (for example, due to a network error or interrupted download), and SHALL report the failure clearly. This applies equally to a manual invocation, the startup invocation, and any recurring invocation while the daemon runs.

#### Scenario: Update fails
- **WHEN** `update-ytdlp` cannot download or install the new release
- **THEN** it exits with a non-zero status and an error message, and the previously installed yt-dlp binary remains usable by `download`

#### Scenario: A recurring update attempt fails
- **WHEN** a recurring automatic update attempt fails while the daemon is running
- **THEN** the previously installed yt-dlp binary remains usable by `download`, and the system still attempts the next recurring update on schedule
