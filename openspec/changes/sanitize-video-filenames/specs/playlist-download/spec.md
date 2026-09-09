## MODIFIED Requirements

### Requirement: Sequential Video Download
The system SHALL download each resolved video by invoking a locally installed `yt-dlp` executable, one video at a time in playlist order, saving output into the given output directory using the filename derived by the `video-naming` capability. The system SHALL NOT pass any format/quality selection flags in this version, relying on `yt-dlp`'s own default behavior for format and quality selection only.

#### Scenario: Multiple videos downloaded in order
- **WHEN** the resolved playlist contains multiple videos
- **THEN** the system invokes `yt-dlp` for the first video, waits for it to finish, then proceeds to the next video, never running two downloads concurrently

#### Scenario: Output directory does not exist
- **WHEN** the given output path does not already exist on disk
- **THEN** the system creates it before starting any downloads
