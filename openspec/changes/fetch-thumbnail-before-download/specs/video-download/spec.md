## MODIFIED Requirements

### Requirement: Per-Playlist Output Directory
The system SHALL save a downloaded video's file inside a dedicated per-video folder, itself located under a directory determined by its owning playlist's or channel's configured storage path, within a single configured root directory. A storage path may contain multiple segments, producing nested subdirectories. The per-video folder's name SHALL be derived the same way the video's filename is derived (its sanitized title, disambiguated on collision — see the `video-naming` capability), so the video's file, its thumbnail, and its metadata file all live together in one folder per video. When a thumbnail was already fetched for the video ahead of its download (see `video-thumbnails`), the download SHALL reuse that same per-video folder rather than deciding a new one.

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

#### Scenario: Video's thumbnail was already fetched ahead of its download
- **WHEN** a video's full download runs and that video already has a recorded thumbnail filename from an earlier thumbnail-only fetch
- **THEN** the system saves the video's file into that thumbnail's own folder instead of resolving a new folder name

#### Scenario: Video has no pre-fetched thumbnail
- **WHEN** a video's full download runs and that video has no recorded thumbnail filename
- **THEN** the system resolves its folder name the same way it always has, including disambiguating a naming collision
