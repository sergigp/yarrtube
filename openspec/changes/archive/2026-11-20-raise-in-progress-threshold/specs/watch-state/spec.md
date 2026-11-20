## ADDED Requirements

### Requirement: Started Playback Position
A saved playback position SHALL count as started when it is greater than 30 seconds and, when the video's duration is known, is also at least 20% of that duration or at least 10 minutes (600 seconds). When the duration is not known, a position greater than 30 seconds SHALL count as started. The duration SHALL be the video's recorded duration, or for a Plex episode the duration Plex reports. A position that does not count as started SHALL still be saved and offered to resume playback from.

#### Scenario: Short share of a video
- **WHEN** a 40-minute video's saved position is 7 minutes
- **THEN** it does not count as started

#### Scenario: 20% of a video
- **WHEN** a 40-minute video's saved position is 8 minutes
- **THEN** it counts as started

#### Scenario: 10 minutes of a long video
- **WHEN** a 2-hour video's saved position is 10 minutes
- **THEN** it counts as started

#### Scenario: Short video past 20% but under 30 seconds
- **WHEN** a 1-minute video's saved position is 25 seconds
- **THEN** it does not count as started

#### Scenario: Unknown duration
- **WHEN** a video with no known duration has a saved position of 31 seconds
- **THEN** it counts as started

## MODIFIED Requirements

### Requirement: Record Playback Progress
The system SHALL provide an HTTP endpoint that, given a stored video's `id` and a playback position in whole seconds (plus, optionally, the duration reported by the player), records playback progress for that video and every other stored copy of its YouTube video. Each report SHALL state whether the client believed the video watched when its playback began (`was_watched`). A report SHALL be stale when it states the video was unwatched but the video is now watched. The video was marked watched after that playback began, so the reported position predates the mark. A stale report SHALL change nothing. A watched video SHALL become unwatched only when a report from a client that believed it watched has a position that counts as started (see "Started Playback Position") and is below 90% of the duration; below that it SHALL stay watched with a saved position of 0. The recorded duration SHALL be used. The reported duration SHALL be used only when no duration is recorded. When included, the reported duration SHALL be a positive number of whole seconds. The endpoint SHALL respond with HTTP status 200 on success, including stale reports, with a body stating whether the video is watched once the report is applied (`{ "watched": true }` or `{ "watched": false }`). It SHALL accept requests sent as a browser beacon on page unload.

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
- **WHEN** a client that believed the video watched records, for a watched 40-minute video, a position of 2 minutes
- **THEN** the video stays watched and its saved position stays 0

#### Scenario: Rewatch passes 10%
- **WHEN** a client that believed the video watched records, for a watched 40-minute video, a position of 7 minutes (past 10% but not started, see "Started Playback Position")
- **THEN** the video stays watched and its saved position stays 0

#### Scenario: Rewatch counts as started
- **WHEN** a client that believed the video watched records, for a watched 40-minute video, a position of 10 minutes (started and below 90%)
- **THEN** the daemon responds with `watched: false`, the video becomes unwatched and its saved position becomes the given position

#### Scenario: Progress under the started threshold is still saved
- **WHEN** a client records a position of 5 minutes for an unwatched 40-minute video
- **THEN** the video stays unwatched and its saved position becomes 300 seconds, so playback can resume there

#### Scenario: Playing on past 90%
- **WHEN** a client that believed the video watched records a position at or above 90% of the duration for a watched video
- **THEN** the video stays watched and its saved position stays 0

#### Scenario: Stale report after the video was marked watched
- **WHEN** a client that believed the video unwatched records a position that counts as started and is below 90% of the duration for a video that is now watched
- **THEN** the daemon responds with HTTP status 200 and `watched: true`, and the video stays watched with a saved position of 0

#### Scenario: Video un-watched elsewhere
- **WHEN** a client that believed the video watched records progress for a video that is now unwatched
- **THEN** the report is applied by the unwatched-video rules

#### Scenario: Unknown video
- **WHEN** a client records progress for a video `id` that is not stored
- **THEN** the daemon responds with HTTP status 400 and records nothing

#### Scenario: Invalid reported duration
- **WHEN** a client records progress with a reported duration of 0 or less
- **THEN** the daemon responds with HTTP status 400 and records nothing

#### Scenario: Missing or invalid position
- **WHEN** a client records progress without a position, or with a negative position
- **THEN** the daemon responds with HTTP status 400 and records nothing

#### Scenario: Missing believed watched state
- **WHEN** a client records progress without stating whether it believed the video watched
- **THEN** the daemon responds with HTTP status 400 and records nothing

### Requirement: Last Played Time
The system SHALL record, for each video, the time playback progress was last recorded for it. Every successful progress report that is not stale SHALL set this time on every stored copy of that YouTube video, whether or not the report changes the video's watched status or saved position. A watched or in-progress state taken from Plex by the Plex watch sync SHALL set this time, on every stored copy, to when Plex last played the episode. A video whose progress has never been recorded SHALL have no last played time. Marking a channel or a single video watched SHALL NOT change any video's last played time.

#### Scenario: Progress sets the last played time
- **WHEN** a client records playback progress for a video
- **THEN** every stored copy of that video has its last played time set to the time of the report

#### Scenario: Progress that doesn't change the watch state still counts as played
- **WHEN** a client that believed the video watched records a position that does not count as started for a watched video
- **THEN** the video stays watched, and its last played time is set to the time of the report

#### Scenario: Playback in Plex counts as played
- **WHEN** the Plex watch sync takes an in-progress state from an episode Plex last played at 18:00
- **THEN** every stored copy of that video has its last played time set to 18:00

#### Scenario: A newly recorded video has never been played
- **WHEN** a video is recorded for a playlist or channel for the first time
- **THEN** it has no last played time

#### Scenario: Marking a channel watched leaves last played times alone
- **WHEN** a client marks a tracked channel as watched
- **THEN** the last played time of each of its videos is unchanged

#### Scenario: Marking a video watched leaves its last played time alone
- **WHEN** a client marks a single video as watched
- **THEN** the last played time of each of its stored copies is unchanged

#### Scenario: A stale report records nothing
- **WHEN** a progress report is stale
- **THEN** no video's last played time changes

#### Scenario: Rejected progress records nothing
- **WHEN** a progress report is rejected as invalid or for an unknown video
- **THEN** no video's last played time changes
