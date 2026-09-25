## ADDED Requirements

### Requirement: Listed Videos Include Sync Time
The system SHALL include each video's recorded sync time in the responses that list the videos of a playlist and of a channel, and SHALL report it as absent for a video that has none.

#### Scenario: Downloaded video is listed
- **WHEN** a client lists the videos of a tracked playlist or channel that has a downloaded video
- **THEN** that video's entry includes its sync time

#### Scenario: Video without a sync time is listed
- **WHEN** a client lists the videos of a tracked playlist or channel that has a video with no recorded sync time
- **THEN** that video's entry reports its sync time as absent
