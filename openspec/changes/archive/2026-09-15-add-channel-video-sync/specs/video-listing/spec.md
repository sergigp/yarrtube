## ADDED Requirements

### Requirement: List Videos For A Channel
The system SHALL provide an HTTP endpoint that, given a tracked channel's handle, returns every video recorded for that channel, including each video's ID, title, status, quality (when downloaded), and filename (when downloaded), ordered by recency (most recently uploaded first).

#### Scenario: Channel has videos
- **WHEN** a client requests the videos of a tracked channel that has one or more recorded videos
- **THEN** the daemon responds with HTTP status 200 and a list containing each of those videos, ordered most recent first

#### Scenario: Channel has no videos
- **WHEN** a client requests the videos of a tracked channel that has no recorded videos
- **THEN** the daemon responds with HTTP status 200 and an empty list

#### Scenario: Channel does not exist
- **WHEN** a client requests the videos of a channel handle that is not currently tracked
- **THEN** the daemon responds with HTTP status 400 and does not return a list
