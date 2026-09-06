## Purpose

Keeps the yt-dlp binary used by the download task current without relying on the host or a package manager, since the container is expected to run unattended on a NAS for long periods during which YouTube's site changes can break older yt-dlp versions.

## ADDED Requirements

### Requirement: Fetch Latest Release
The system SHALL provide an `update-ytdlp` task that downloads the latest stable yt-dlp release binary for Linux and replaces the binary used by the `download` task.

#### Scenario: Update available
- **WHEN** `update-ytdlp` is run and a newer yt-dlp release is available
- **THEN** the task downloads that release and replaces the local yt-dlp binary, and subsequent `download` invocations use it

#### Scenario: Already up to date
- **WHEN** `update-ytdlp` is run and the local yt-dlp binary is already the latest release
- **THEN** the task completes successfully and the `download` task continues to work

### Requirement: Failure Isolation
The system SHALL leave the existing yt-dlp binary usable if `update-ytdlp` fails partway (for example, due to a network error or interrupted download), and SHALL report the failure clearly.

#### Scenario: Update fails
- **WHEN** `update-ytdlp` cannot download or install the new release
- **THEN** it exits with a non-zero status and an error message, and the previously installed yt-dlp binary remains usable by `download`

### Requirement: Manual and Automatic Invocation
The system SHALL support running the yt-dlp update both as an explicit ad hoc command and automatically once each time the daemon starts.

#### Scenario: Manual invocation
- **WHEN** an operator runs `update-ytdlp` directly
- **THEN** it performs the update independent of whether the daemon is running

#### Scenario: Automatic invocation at daemon startup
- **WHEN** the daemon starts
- **THEN** it performs the same update behavior as running `update-ytdlp` manually
