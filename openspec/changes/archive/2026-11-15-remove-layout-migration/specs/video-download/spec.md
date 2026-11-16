## MODIFIED Requirements

### Requirement: Per-Playlist Output Directory
The system SHALL save a downloaded video's file inside the `Season <season>` folder of its show directory, the show directory being determined by its owning playlist's or channel's configured storage path within a single configured root directory. A storage path may contain multiple segments, producing nested subdirectories. The file's name SHALL be the video's episode base name (see `video-naming` and `tv-show-layout`) with the media extension, so the video's file, its thumbnail, and its metadata file sit next to each other under one base name. The download SHALL reuse the episode number already recorded on the video (a playlist video's from when it was first stored, or one recorded by an earlier thumbnail-ahead fetch, see `video-thumbnails`), and SHALL assign a channel video's number when it has none yet.

#### Scenario: Video downloaded
- **WHEN** a video belonging to a playlist finishes downloading
- **THEN** its file is saved as `Season <season>/<episode base>.mp4` under the configured root directory, in the subdirectory (or nested subdirectories) identified by its playlist's storage path

#### Scenario: Video downloaded for a channel
- **WHEN** a video belonging to a channel finishes downloading
- **THEN** its file is saved as `Season <season>/<episode base>.mp4` under the configured root directory, in the subdirectory (or nested subdirectories) identified by its channel's storage path

#### Scenario: Root directory not configured
- **WHEN** no root output directory has been explicitly configured
- **THEN** the system uses a default root directory

#### Scenario: Video downloaded under a legacy flat layout
- **WHEN** a video was downloaded before the TV-show layout was introduced and its recorded file still exists at its original location (a per-video folder or a flat file under the source directory)
- **THEN** the system continues to treat that file as valid, keeps serving it where it is

#### Scenario: Video's thumbnail was already fetched ahead of its download
- **WHEN** a video's full download runs and that video already has a recorded episode number and thumbnail filename from an earlier thumbnail-only fetch
- **THEN** the system saves the video's file under that same episode base name, next to the thumbnail

#### Scenario: Video has no pre-fetched thumbnail
- **WHEN** a channel video's full download runs and that video has no recorded episode number
- **THEN** the system assigns its number from the publish timestamp the downloader reports and saves the file under the resulting episode base name

#### Scenario: Playlist video downloaded
- **WHEN** a playlist video's full download runs
- **THEN** the system saves the file under the episode number recorded when the playlist first stored the video, whatever the reported publish timestamp

#### Scenario: Finished download cannot be placed
- **WHEN** numbering, moving or recording a finished download fails
- **THEN** the system discards the staged files and marks the video errored like a failed download (retrying, or errored on the last attempt), keeping any episode number already assigned

