## MODIFIED Requirements

### Requirement: Per-Playlist Output Directory
The system SHALL save a downloaded video's file under a directory determined by its playlist's configured storage path, within a single configured root directory. A storage path may contain multiple segments, producing nested subdirectories.

#### Scenario: Video downloaded
- **WHEN** a video finishes downloading
- **THEN** its file is saved under the configured root directory, in the subdirectory (or nested subdirectories) identified by its playlist's storage path

#### Scenario: Root directory not configured
- **WHEN** no root output directory has been explicitly configured
- **THEN** the system uses a default root directory

## ADDED Requirements

### Requirement: Downloaded Quality Is Recorded
The system SHALL record, on the video itself, the quality tier (`high`, `mid`, or `low`) it was actually downloaded at whenever a download succeeds. A video that has not yet completed a successful download SHALL have no recorded quality.

#### Scenario: Download succeeds
- **WHEN** a video download completes successfully at a given quality tier
- **THEN** the video's recorded quality becomes that tier

#### Scenario: Video not yet successfully downloaded
- **WHEN** a video is pending, in progress, or has only failed attempts so far
- **THEN** the video has no recorded quality
