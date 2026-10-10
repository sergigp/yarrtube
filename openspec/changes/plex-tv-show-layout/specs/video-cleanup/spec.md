## MODIFIED Requirements

### Requirement: File Deletion Triggered By Video Removal
The system SHALL delete a video's downloaded file, its recorded thumbnail file if any, and its episode NFO if any, from disk as soon as that video is removed from its tracked playlist or tracked channel, provided the video had reached the downloaded status. The show's `tvshow.nfo`, its poster and the season folder SHALL NOT be deleted with a video.

#### Scenario: Downloaded video removed from its playlist
- **WHEN** a video with status downloaded is removed from its playlist
- **THEN** the system deletes that video's file, its thumbnail file if it has one, and its NFO if present, from disk, and leaves the show files and the season folder in place

#### Scenario: Downloaded video removed from its channel
- **WHEN** a video with status downloaded ages out of its channel's most recent `video_limit` uploads (or is otherwise no longer among them)
- **THEN** the system deletes that video's file, its thumbnail file if it has one, and its NFO if present, from disk

#### Scenario: Never-downloaded video removed from its playlist
- **WHEN** a video that never reached the downloaded status is removed from its playlist
- **THEN** the system does not attempt any file deletion for that video

#### Scenario: Never-downloaded video removed from its channel
- **WHEN** a video that never reached the downloaded status is removed from its channel
- **THEN** the system does not attempt any file deletion for that video

### Requirement: File Located By Its Recorded Filename
The system SHALL locate a removed video's file, and separately its
thumbnail file, each by its own recorded filename (the exact path recorded
when the video was downloaded, relative to the show directory), rather than
by re-deriving either name from its title. Its NFO SHALL be located as the
recorded filename with the `.nfo` extension. A video still stored in a
legacy per-video folder SHALL have that folder deleted as a whole.

#### Scenario: File found by recorded filename
- **WHEN** a removed video has a recorded filename and its owning playlist's or channel's show directory contains a file at that exact path
- **THEN** the system deletes that file and the NFO sharing its base name

#### Scenario: Thumbnail found by recorded thumbnail filename
- **WHEN** a removed video has a recorded thumbnail filename and its owning playlist's or channel's show directory contains a file at that exact path
- **THEN** the system deletes that file

#### Scenario: No recorded filename
- **WHEN** a removed video has no recorded filename because it was never successfully downloaded
- **THEN** the system does not attempt any file deletion for it

#### Scenario: No recorded thumbnail filename
- **WHEN** a removed video has no recorded thumbnail filename (it was never written, or the video was never successfully downloaded)
- **THEN** the system does not attempt any thumbnail file deletion for it

#### Scenario: Recorded filename not found on disk
- **WHEN** a removed video has a recorded filename but no file at that exact path exists in its show directory
- **THEN** the system does not treat this as an error, and no file is deleted

#### Scenario: Legacy per-video folder
- **WHEN** a removed video's recorded filename still points into a per-video folder of the previous layout
- **THEN** the system deletes that folder and everything in it
