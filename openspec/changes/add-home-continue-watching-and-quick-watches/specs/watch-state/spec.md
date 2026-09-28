## ADDED Requirements

### Requirement: Last Played Time
The system SHALL record, for each video, the time playback progress was last recorded for it. Every successful progress report SHALL set this time on every stored copy of that YouTube video, whether or not the report changes the video's watched status or saved position. A video whose progress has never been recorded SHALL have no last played time. Marking a channel watched SHALL NOT change any video's last played time.

#### Scenario: Progress sets the last played time
- **WHEN** a client records playback progress for a video
- **THEN** every stored copy of that video has its last played time set to the time of the report

#### Scenario: Progress that doesn't change the watch state still counts as played
- **WHEN** a client records a position at or below 10% of the duration for a watched video
- **THEN** the video stays watched, and its last played time is set to the time of the report

#### Scenario: A newly recorded video has never been played
- **WHEN** a video is recorded for a playlist or channel for the first time
- **THEN** it has no last played time

#### Scenario: Marking a channel watched leaves last played times alone
- **WHEN** a client marks a tracked channel as watched
- **THEN** the last played time of each of its videos is unchanged

#### Scenario: Rejected progress records nothing
- **WHEN** a progress report is rejected as invalid or for an unknown video
- **THEN** no video's last played time changes
