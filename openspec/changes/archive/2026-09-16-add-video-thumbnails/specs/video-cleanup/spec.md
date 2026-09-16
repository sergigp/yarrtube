## MODIFIED Requirements

### Requirement: File Deletion Triggered By Video Removal
The system SHALL delete a video's downloaded file, and its recorded thumbnail file if any, from disk as soon as that video is removed from its tracked playlist or tracked channel, provided the video had reached the downloaded status. The system SHALL NOT attempt any file deletion for a video that never finished downloading.

#### Scenario: Downloaded video removed from its playlist
- **WHEN** a video with status downloaded is removed from its playlist
- **THEN** the system deletes that video's file, and its thumbnail file if it has one, from disk

#### Scenario: Downloaded video removed from its channel
- **WHEN** a video with status downloaded ages out of its channel's most recent `video_limit` uploads (or is otherwise no longer among them)
- **THEN** the system deletes that video's file, and its thumbnail file if it has one, from disk

#### Scenario: Never-downloaded video removed from its playlist
- **WHEN** a video that never reached the downloaded status is removed from its playlist
- **THEN** the system does not attempt any file deletion for that video

#### Scenario: Never-downloaded video removed from its channel
- **WHEN** a video that never reached the downloaded status is removed from its channel
- **THEN** the system does not attempt any file deletion for that video

### Requirement: File Located By Its Recorded Filename
The system SHALL locate a removed video's file, and separately its
thumbnail file, each by its own recorded filename (the exact name recorded
when the video was downloaded), rather than by re-deriving either name from
its title.

#### Scenario: File found by recorded filename
- **WHEN** a removed video has a recorded filename and its owning playlist's or channel's output directory contains a file with that exact name
- **THEN** the system deletes that file

#### Scenario: Thumbnail found by recorded thumbnail filename
- **WHEN** a removed video has a recorded thumbnail filename and its owning playlist's or channel's output directory contains a file with that exact name
- **THEN** the system deletes that file

#### Scenario: No recorded filename
- **WHEN** a removed video has no recorded filename because it was never successfully downloaded
- **THEN** the system does not attempt any file deletion for it

#### Scenario: No recorded thumbnail filename
- **WHEN** a removed video has no recorded thumbnail filename (it was never written, or the video was never successfully downloaded)
- **THEN** the system does not attempt any thumbnail file deletion for it

#### Scenario: Recorded filename not found on disk
- **WHEN** a removed video has a recorded filename but no file with that exact name exists in its owning playlist's or channel's output directory
- **THEN** the system does not treat this as an error, and no file is deleted
