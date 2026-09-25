## ADDED Requirements

### Requirement: Sync Time Is Recorded
The system SHALL record, on the video itself, the time its download last succeeded (its sync time). A video that has not completed a successful download since it was created or last reset for redownload SHALL have no recorded sync time. Recording a thumbnail, a watch-state change, or any other update SHALL NOT change a video's sync time.

#### Scenario: Download succeeds
- **WHEN** a video download completes successfully
- **THEN** the video's recorded sync time becomes the time the download completed

#### Scenario: Video not yet successfully downloaded
- **WHEN** a video is pending, in progress, or has only failed attempts so far
- **THEN** the video has no recorded sync time

#### Scenario: Video reset for redownload
- **WHEN** a downloaded video is reset for redownload
- **THEN** the video has no recorded sync time until its next download succeeds

#### Scenario: Thumbnail recorded after download
- **WHEN** a thumbnail is recorded for a video that was already downloaded
- **THEN** the video's recorded sync time is unchanged

#### Scenario: Video downloaded before sync time was tracked
- **WHEN** the system upgrades storage that holds videos already downloaded
- **THEN** each of those videos gets its last update time as its recorded sync time, and every other video has none
