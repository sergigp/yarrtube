## ADDED Requirements

### Requirement: Playlist Directory Deleted When Its Playlist Is Deleted
The system SHALL delete a playlist's entire output directory from disk once that playlist has been deleted, regardless of what videos it contained, their download status, or whether any files were ever written to it. This is separate from, and not conditioned on, per-video file deletion (see the video-removal requirements above), since deleting a playlist removes every video record for it in the same operation rather than removing them one at a time.

#### Scenario: Deleted playlist had downloaded videos
- **WHEN** a playlist with one or more downloaded videos is deleted
- **THEN** the system deletes that playlist's output directory, and every file in it, from disk

#### Scenario: Deleted playlist had no downloaded videos
- **WHEN** a playlist with no downloaded videos (or no videos at all) is deleted
- **THEN** the system does not treat a missing or empty output directory as an error

#### Scenario: Output directory already absent
- **WHEN** a deleted playlist's output directory does not exist on disk at the time this cleanup runs
- **THEN** the system does not treat this as an error, and makes no filesystem changes
