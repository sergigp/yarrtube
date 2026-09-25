## Purpose

Tracks whether each video has been watched, and where playback stopped, so the web UI can resume videos, mark watched ones, and count unwatched uploads per channel.

## ADDED Requirements

### Requirement: Watch State Per YouTube Video
The system SHALL record, for each video, whether it has been watched and its saved playback position in whole seconds. A video that has never been played SHALL be unwatched with a saved position of 0. Watch state SHALL belong to the YouTube video: every change SHALL apply to every stored copy of that YouTube video, whichever playlist or channel tracks it.

#### Scenario: A newly recorded video is unwatched
- **WHEN** a video is recorded for a playlist or channel for the first time
- **THEN** it is unwatched with a saved playback position of 0

#### Scenario: A change applies to every stored copy
- **WHEN** the same YouTube video is tracked by both a channel and a playlist, and its watch state changes
- **THEN** both stored copies report the same new watch state

#### Scenario: Watch state survives a redownload
- **WHEN** a watched video's file goes missing and the video is scheduled for a fresh download
- **THEN** the video stays watched

### Requirement: Record Playback Progress
The system SHALL provide an HTTP endpoint that, given a YouTube video ID and a playback position in whole seconds (plus, optionally, the duration reported by the player), records playback progress for every stored copy of that video. The recorded duration SHALL be used. The reported duration SHALL be used only when no duration is recorded. When included, the reported duration SHALL be a positive number of whole seconds. The endpoint SHALL respond with HTTP status 204 on success. It SHALL accept requests sent as a browser beacon on page unload.

#### Scenario: Progress on an unwatched video
- **WHEN** a client records a position below 90% of the video's duration for an unwatched video
- **THEN** the daemon responds with HTTP status 204, the video stays unwatched, and its saved position becomes the given position

#### Scenario: Progress reaches 90%
- **WHEN** a client records a position at or above 90% of the video's duration for an unwatched video
- **THEN** the video becomes watched and its saved position resets to 0

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
- **THEN** the video becomes unwatched and its saved position becomes the given position

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

### Requirement: Mark A Channel Watched
The system SHALL provide an HTTP endpoint that, given a tracked channel's handle, marks every downloaded video of that channel as watched, including every other stored copy of those videos. Videos in the channel that have not finished downloading SHALL be left unchanged. The endpoint SHALL respond with HTTP status 204.

#### Scenario: Marking a channel watched
- **WHEN** a client marks a tracked channel as watched
- **THEN** the daemon responds with HTTP status 204 and every downloaded video of the channel is watched

#### Scenario: Videos still downloading are left unchanged
- **WHEN** a client marks a channel as watched while some of its videos are pending or downloading
- **THEN** those videos stay unwatched

#### Scenario: Channel does not exist
- **WHEN** a client marks as watched a channel handle that is not currently tracked
- **THEN** the daemon responds with HTTP status 400 and changes nothing

#### Scenario: Invalid handle
- **WHEN** a client marks as watched a value that is not a valid channel handle
- **THEN** the daemon responds with HTTP status 400 and changes nothing
