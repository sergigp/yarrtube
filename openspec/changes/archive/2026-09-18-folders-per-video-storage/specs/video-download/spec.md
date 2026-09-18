## MODIFIED Requirements

### Requirement: Per-Playlist Output Directory
The system SHALL save a downloaded video's file inside a dedicated per-video folder, itself located under a directory determined by its owning playlist's or channel's configured storage path, within a single configured root directory. A storage path may contain multiple segments, producing nested subdirectories. The per-video folder's name SHALL be derived the same way the video's filename is derived (its sanitized title, disambiguated on collision — see the `video-naming` capability), so the video's file, its thumbnail, and its metadata file all live together in one folder per video.

#### Scenario: Video downloaded
- **WHEN** a video belonging to a playlist finishes downloading
- **THEN** its file is saved in its own folder, under the configured root directory, in the subdirectory (or nested subdirectories) identified by its playlist's storage path

#### Scenario: Video downloaded for a channel
- **WHEN** a video belonging to a channel finishes downloading
- **THEN** its file is saved in its own folder, under the configured root directory, in the subdirectory (or nested subdirectories) identified by its channel's storage path

#### Scenario: Root directory not configured
- **WHEN** no root output directory has been explicitly configured
- **THEN** the system uses a default root directory

#### Scenario: Video downloaded under a legacy flat layout
- **WHEN** a video was downloaded before per-video folders were introduced, and its recorded file still exists at its original flat location directly under the playlist's or channel's output directory
- **THEN** the system continues to treat that file as valid and does not require it to be moved into a per-video folder

## ADDED Requirements

### Requirement: Metadata File Placeholder
The system SHALL create an empty `meta.nfo` file in a video's own folder whenever that video's download succeeds. This change does not define any content for that file — populating it is left to a future change.

#### Scenario: Video downloaded successfully
- **WHEN** a video download completes successfully
- **THEN** an empty `meta.nfo` file exists in that video's own folder
