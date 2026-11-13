# Spec Delta

## MODIFIED Requirements

### Requirement: Last Played Time
The system SHALL record, for each video, the time playback progress was last recorded for it. Every successful progress report that is not stale SHALL set this time on every stored copy of that YouTube video, whether or not the report changes the video's watched status or saved position. A watched or in-progress state taken from Plex by the Plex watch sync SHALL set this time, on every stored copy, to when Plex last played the episode. A video whose progress has never been recorded SHALL have no last played time. Marking a channel or a single video watched SHALL NOT change any video's last played time.

#### Scenario: Progress sets the last played time
- **WHEN** a client records playback progress for a video
- **THEN** every stored copy of that video has its last played time set to the time of the report

#### Scenario: Progress that doesn't change the watch state still counts as played
- **WHEN** a client that believed the video watched records a position at or below 10% of the duration for a watched video
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
