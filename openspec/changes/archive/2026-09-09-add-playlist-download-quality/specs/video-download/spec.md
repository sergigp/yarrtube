## ADDED Requirements

### Requirement: Download Applies Playlist Quality
The system SHALL download a video's file using the resolution associated with its playlist's configured quality tier (`high`, `mid`, or `low`), preferring an mp4 container with h264 video and aac audio at every tier, and SHALL fall back to the best available stream within that resolution instead of failing the download outright when no stream matches the mp4/h264/aac preference.

#### Scenario: Playlist configured for high quality
- **WHEN** a video belonging to a playlist configured with quality `high` is downloaded
- **THEN** the system downloads it with no resolution cap, preferring an mp4/h264/aac stream

#### Scenario: Playlist configured for mid quality
- **WHEN** a video belonging to a playlist configured with quality `mid` is downloaded
- **THEN** the system downloads it capped at 720p, preferring an mp4/h264/aac stream

#### Scenario: Playlist configured for low quality
- **WHEN** a video belonging to a playlist configured with quality `low` is downloaded
- **THEN** the system downloads it capped at 480p, preferring an mp4/h264/aac stream

#### Scenario: No stream matches the preferred container/codec
- **WHEN** a video has no available stream in mp4/h264/aac at or below its playlist's resolution cap
- **THEN** the system downloads the best available stream within that resolution cap instead of failing the download
