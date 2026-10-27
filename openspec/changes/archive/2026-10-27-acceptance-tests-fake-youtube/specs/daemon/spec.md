## ADDED Requirements

### Requirement: Configurable YouTube Data API Endpoint
The system SHALL send its YouTube Data API requests to `https://www.googleapis.com/youtube/v3` unless an alternative base URL is configured, in which case every YouTube Data API request SHALL go to that base URL instead. The setting exists to run the daemon against a fake YouTube.

#### Scenario: Default endpoint
- **WHEN** no YouTube Data API base URL is configured
- **THEN** playlist, video and channel lookups go to `https://www.googleapis.com/youtube/v3`

#### Scenario: Configured endpoint
- **WHEN** a base URL is configured
- **THEN** every playlist, video and channel lookup goes to that base URL, with the same paths and parameters

### Requirement: Configurable Background Poll Interval
The system SHALL check for due tasks and unhandled domain events every 5 seconds unless another interval is configured, in which case both background loops SHALL use that interval. The setting exists so tests against a fake YouTube don't wait seconds between pipeline steps.

#### Scenario: Default interval
- **WHEN** no background poll interval is configured, or the configured value is not a positive number of milliseconds
- **THEN** the task executor and the event consumer each check for work every 5 seconds

#### Scenario: Configured interval
- **WHEN** a background poll interval of 250 milliseconds is configured
- **THEN** the task executor and the event consumer each check for work every 250 milliseconds

## MODIFIED Requirements

### Requirement: Startup Self-Update
The system SHALL attempt to update its yt-dlp binary once each time the daemon starts, using the same behavior as the `update-ytdlp` task, unless the yt-dlp self-update is turned off.

#### Scenario: Successful startup update
- **WHEN** the daemon starts and the yt-dlp update succeeds
- **THEN** the daemon logs the update result and continues starting up

#### Scenario: Startup update fails
- **WHEN** the daemon starts and the yt-dlp update fails (for example, due to no network access)
- **THEN** the daemon logs the failure and continues starting up rather than exiting, so a temporary network issue does not prevent the container from running

#### Scenario: Self-update turned off
- **WHEN** the daemon starts with the yt-dlp self-update turned off
- **THEN** it does not update or replace its yt-dlp binary, and logs that the self-update is off
