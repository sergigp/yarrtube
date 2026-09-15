## MODIFIED Requirements

### Requirement: Download Triggered By Video Addition
The system SHALL automatically begin downloading a video as soon as it is newly added to a tracked playlist or a tracked channel, without requiring manual intervention.

#### Scenario: New video added to a tracked playlist
- **WHEN** a video is newly added to a tracked playlist
- **THEN** the system downloads that video without any manual action

#### Scenario: New video added to a tracked channel
- **WHEN** a video is newly added to a tracked channel (its `video_limit` most recent uploads)
- **THEN** the system downloads that video without any manual action

### Requirement: Automatic Retry On Download Failure
The system SHALL automatically retry a failed video download a bounded number of times before giving up on that attempt sequence, without any manual action. Exhausting that bounded sequence SHALL NOT prevent the video from being attempted again later by playlist or channel reconciliation.

#### Scenario: Transient download failure
- **WHEN** a video download attempt fails and retries remain
- **THEN** the system automatically attempts the download again after a delay

#### Scenario: Retries exhausted
- **WHEN** a video download has failed on every allowed attempt
- **THEN** the system stops attempting that download within that attempt sequence, though the video may be attempted again later by playlist or channel reconciliation

### Requirement: Per-Playlist Output Directory
The system SHALL save a downloaded video's file under a directory determined by its owning playlist's or channel's configured storage path, within a single configured root directory. A storage path may contain multiple segments, producing nested subdirectories.

#### Scenario: Video downloaded
- **WHEN** a video belonging to a playlist finishes downloading
- **THEN** its file is saved under the configured root directory, in the subdirectory (or nested subdirectories) identified by its playlist's storage path

#### Scenario: Video downloaded for a channel
- **WHEN** a video belonging to a channel finishes downloading
- **THEN** its file is saved under the configured root directory, in the subdirectory (or nested subdirectories) identified by its channel's storage path

#### Scenario: Root directory not configured
- **WHEN** no root output directory has been explicitly configured
- **THEN** the system uses a default root directory

### Requirement: Download Skipped For a Deleted Playlist
The system SHALL NOT attempt to download a video, or treat it as an error, when the download is attempted after the video's owning playlist or channel no longer exists.

#### Scenario: Playlist deleted before its video's download runs
- **WHEN** a video's download is attempted for a playlist that no longer exists
- **THEN** the system makes no download attempt and does not change that video's status

#### Scenario: Channel deleted before its video's download runs
- **WHEN** a video's download is attempted for a channel that no longer exists
- **THEN** the system makes no download attempt and does not change that video's status

### Requirement: Download Applies Playlist Quality
The system SHALL download a video's file using the resolution associated with its owning playlist's or channel's configured quality tier (`high`, `mid`, or `low`), preferring an mp4 container with h264 video and aac audio at every tier, and SHALL fall back to the best available stream within that resolution instead of failing the download outright when no stream matches the mp4/h264/aac preference.

#### Scenario: Playlist configured for high quality
- **WHEN** a video belonging to a playlist configured with quality `high` is downloaded
- **THEN** the system downloads it with no resolution cap, preferring an mp4/h264/aac stream

#### Scenario: Playlist configured for mid quality
- **WHEN** a video belonging to a playlist configured with quality `mid` is downloaded
- **THEN** the system downloads it capped at 720p, preferring an mp4/h264/aac stream

#### Scenario: Playlist configured for low quality
- **WHEN** a video belonging to a playlist configured with quality `low` is downloaded
- **THEN** the system downloads it capped at 480p, preferring an mp4/h264/aac stream

#### Scenario: Channel configured for a quality tier
- **WHEN** a video belonging to a channel configured with a given quality tier is downloaded
- **THEN** the system applies that tier's resolution cap and mp4/h264/aac preference the same way it does for a playlist

#### Scenario: No stream matches the preferred container/codec
- **WHEN** a video has no available stream in mp4/h264/aac at or below its owning playlist's or channel's resolution cap
- **THEN** the system downloads the best available stream within that resolution cap instead of failing the download
