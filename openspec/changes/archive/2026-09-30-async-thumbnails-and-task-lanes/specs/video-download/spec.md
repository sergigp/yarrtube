## ADDED Requirements

### Requirement: Concurrent Downloads Use Distinct Folders
The system SHALL give every video its own per-video folder even when
downloads of different videos run at the same time. Two videos downloading
concurrently into the same playlist's or channel's output directory SHALL
NOT end up sharing a folder, even when their titles produce the same folder
name. The collision SHALL be resolved the same way as any other folder name
collision (see `video-naming`).

#### Scenario: Two videos with the same title download at the same time
- **WHEN** two different videos with the same title, in the same playlist or channel, are downloaded concurrently and neither has a pre-fetched thumbnail folder
- **THEN** each video's file, thumbnail, and metadata are saved in a different folder, and each video's recorded filename points into its own folder

### Requirement: Download Finished For a Deleted Video Leaves No Files
The system SHALL remove the per-video folder a download created when, by the
time that download finishes, the video's record (or its owning playlist or
channel) no longer exists. A deletion that happens while a download is
running SHALL NOT leave behind a folder that no stored video accounts for.

#### Scenario: Playlist deleted while one of its videos is downloading
- **WHEN** a playlist is deleted, and its output directory removed, while one of its videos is still downloading
- **THEN** once that download finishes, the system removes the folder the download wrote, and does not record the download on any video

#### Scenario: Video removed from its playlist while downloading
- **WHEN** a video's record is removed while that video is still downloading
- **THEN** once that download finishes, the system removes the folder the download wrote

#### Scenario: Video still exists when its download finishes
- **WHEN** a download finishes and the video's record still exists
- **THEN** the system records the download as usual and keeps its folder
