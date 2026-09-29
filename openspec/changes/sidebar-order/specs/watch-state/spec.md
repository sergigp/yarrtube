## MODIFIED Requirements

### Requirement: Record Playback Progress
The system SHALL provide an HTTP endpoint that, given a YouTube video ID and a playback position in whole seconds (plus, optionally, the duration reported by the player), records playback progress for every stored copy of that video. The recorded duration SHALL be used. The reported duration SHALL be used only when no duration is recorded. When included, the reported duration SHALL be a positive number of whole seconds. The endpoint SHALL respond with HTTP status 200 on success, with a body stating whether the video is watched once the report is applied (`{ "watched": true }` or `{ "watched": false }`). It SHALL accept requests sent as a browser beacon on page unload.

#### Scenario: Progress on an unwatched video
- **WHEN** a client records a position below 90% of the video's duration for an unwatched video
- **THEN** the daemon responds with HTTP status 200 and `watched: false`, the video stays unwatched, and its saved position becomes the given position

#### Scenario: Progress reaches 90%
- **WHEN** a client records a position at or above 90% of the video's duration for an unwatched video
- **THEN** the daemon responds with `watched: true`, the video becomes watched and its saved position resets to 0

#### Scenario: Duration only known by the player
- **WHEN** a client records progress for a video with no recorded duration and includes the player's duration
- **THEN** the 90% rule is applied against the player's duration

#### Scenario: Duration unknown
- **WHEN** a client records progress for a video with no recorded duration and no player duration
- **THEN** the saved position becomes the given position and the video's watched status does not change

#### Scenario: Early in a rewatch
- **WHEN** a client records a position at or below 10% of the duration for a watched video
- **THEN** the video stays watched and its saved position stays 0

#### Scenario: Rewatch passes 10%
- **WHEN** a client records a position above 10% and below 90% of the duration for a watched video
- **THEN** the daemon responds with `watched: false`, the video becomes unwatched and its saved position becomes the given position

#### Scenario: Playing on past 90%
- **WHEN** a client records a position at or above 90% of the duration for a watched video
- **THEN** the video stays watched and its saved position stays 0

#### Scenario: Unknown video
- **WHEN** a client records progress for a YouTube video ID that no playlist or channel tracks
- **THEN** the daemon responds with HTTP status 400 and records nothing

#### Scenario: Invalid reported duration
- **WHEN** a client records progress with a reported duration of 0 or less
- **THEN** the daemon responds with HTTP status 400 and records nothing

#### Scenario: Missing or invalid position
- **WHEN** a client records progress without a position, or with a negative position
- **THEN** the daemon responds with HTTP status 400 and records nothing
