## ADDED Requirements

### Requirement: Download Skipped For an Already-Settled Video
The system SHALL, when a download task runs for a video whose download status is already Downloaded, Excluded or Errored, complete the task without downloading and without changing the video or its files.

#### Scenario: Download task for an already-downloaded video
- **WHEN** a download task runs for a video whose status is Downloaded
- **THEN** no download is attempted, the video is left unchanged, and the task completes without error

#### Scenario: Download task for an excluded video
- **WHEN** a download task runs for a video whose status is Excluded
- **THEN** no download is attempted, the video is left unchanged, and the task completes without error

#### Scenario: Download task for a permanently errored video
- **WHEN** a download task runs for a video whose status is Errored
- **THEN** no download is attempted, the video is left unchanged, and the task completes without error
