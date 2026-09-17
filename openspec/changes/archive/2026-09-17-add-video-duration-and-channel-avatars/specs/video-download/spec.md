## ADDED Requirements

### Requirement: Downloaded Duration Is Recorded
The system SHALL record, on the video itself, its duration in seconds, whenever a download succeeds and a duration is available for it. A video that has not yet completed a successful download SHALL have no recorded duration.

#### Scenario: Download succeeds with a known duration
- **WHEN** a video download completes successfully and a duration is available for it
- **THEN** the video's recorded duration becomes that duration, in seconds

#### Scenario: Download succeeds without a known duration
- **WHEN** a video download completes successfully but no duration could be determined for it
- **THEN** the video has no recorded duration

#### Scenario: Video not yet successfully downloaded
- **WHEN** a video is pending, in progress, or has only failed attempts so far
- **THEN** the video has no recorded duration

#### Scenario: Video re-downloaded after being reset
- **WHEN** a video that previously had a recorded duration is reset and successfully downloaded again
- **THEN** its recorded duration reflects the new download (a fresh duration, or none, replacing the prior one)
