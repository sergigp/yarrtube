## MODIFIED Requirements

### Requirement: List Channels
The system SHALL provide an HTTP endpoint that returns every currently stored channel. Each returned channel SHALL include only its handle, name, path and avatar filename (when present), plus its count of unwatched videos, counting only videos that have finished downloading.

#### Scenario: Channels exist
- **WHEN** one or more channels have been created
- **THEN** the system returns all of them, each with only its handle, name, path, avatar filename (when present), and unwatched video count

#### Scenario: No channels exist
- **WHEN** no channels have been created
- **THEN** the system returns an empty list rather than an error

#### Scenario: Unwatched count only includes downloaded, unwatched videos
- **WHEN** a channel has downloaded videos that are unwatched, downloaded videos that are watched, and videos that have not finished downloading
- **THEN** its unwatched video count equals the number of downloaded, unwatched videos only
