## MODIFIED Requirements

### Requirement: Per-Playlist Output Directory
The system SHALL save a downloaded video's file inside the `Season <season>` folder of its show directory, the show directory being determined by its owning playlist's or channel's configured storage path within a single configured root directory. A storage path may contain multiple segments, producing nested subdirectories. The file's name SHALL be the video's episode base name (see `video-naming` and `tv-show-layout`) with the media extension, so the video's file, its thumbnail, and its metadata file sit next to each other under one base name. The download SHALL assign the video's episode number when it has none yet, and SHALL reuse the number already recorded by an earlier thumbnail-ahead fetch (see `video-thumbnails`).

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
- **THEN** the system continues to treat that file as valid, keeps serving it, and leaves moving it to the `migrate-layout` subcommand

#### Scenario: Video's thumbnail was already fetched ahead of its download
- **WHEN** a video's full download runs and that video already has a recorded episode number and thumbnail filename from an earlier thumbnail-only fetch
- **THEN** the system saves the video's file under that same episode base name, next to the thumbnail

#### Scenario: Video has no pre-fetched thumbnail
- **WHEN** a video's full download runs and that video has no recorded episode number
- **THEN** the system assigns its number from the publish timestamp the downloader reports and saves the file under the resulting episode base name

### Requirement: Concurrent Downloads Use Distinct Folders
The system SHALL give every video its own episode base name even when
downloads of different videos run at the same time. Two videos downloading
concurrently into the same show SHALL NOT end up sharing a base name, even
when their titles are identical and they were published on the same day,
because same-day indexes are assigned from the recorded numbers and never
reused.

#### Scenario: Two videos with the same title download at the same time
- **WHEN** two different videos with the same title and publish day, in the same playlist or channel, are downloaded concurrently
- **THEN** each video's file, thumbnail, and metadata are saved under a different episode base name, and each video's recorded filename points at its own file

### Requirement: Download Finished For a Deleted Video Leaves No Files
The system SHALL remove the files a download created (its media file,
thumbnail and NFO) when, by the time that download finishes, the video's
record (or its owning playlist or channel) no longer exists. A deletion that
happens while a download is running SHALL NOT leave behind files that no
stored video accounts for. Season folders and show files are left alone.

#### Scenario: Playlist deleted while one of its videos is downloading
- **WHEN** a playlist is deleted, and its output directory removed, while one of its videos is still downloading
- **THEN** once that download finishes, the system removes the files the download wrote, and does not record the download on any video

#### Scenario: Video removed from its playlist while downloading
- **WHEN** a video's record is removed while that video is still downloading
- **THEN** once that download finishes, the system removes the files the download wrote

#### Scenario: Video still exists when its download finishes
- **WHEN** a download finishes and the video's record still exists
- **THEN** the system records the download as usual and keeps its files
