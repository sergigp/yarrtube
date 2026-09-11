## REMOVED Requirements

### Requirement: File Located By Matching Sanitized Title
**Reason**: Replaced by exact matching on the video's recorded filename
(see `video-download`'s "Downloaded Filename Is Recorded"), now that a
download records the real filename it produced instead of it being
re-derived by guessing from the title.
**Migration**: See this capability's "File Located By Its Recorded
Filename" requirement.

## ADDED Requirements

### Requirement: File Located By Its Recorded Filename
The system SHALL locate a removed video's file by its recorded filename
(the exact name recorded when the video was downloaded), rather than by
re-deriving a filename from its title.

#### Scenario: File found by recorded filename
- **WHEN** a removed video has a recorded filename and its playlist's output directory contains a file with that exact name
- **THEN** the system deletes that file

#### Scenario: No recorded filename
- **WHEN** a removed video has no recorded filename because it was never successfully downloaded
- **THEN** the system does not attempt any file deletion for it

#### Scenario: Recorded filename not found on disk
- **WHEN** a removed video has a recorded filename but no file with that exact name exists in its playlist's output directory
- **THEN** the system does not treat this as an error, and no file is deleted
