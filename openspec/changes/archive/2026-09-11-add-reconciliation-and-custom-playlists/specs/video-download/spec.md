## ADDED Requirements

### Requirement: Downloaded Filename Is Recorded
The system SHALL record, on the video itself, the exact filename its file
was saved under on disk, whenever a download succeeds. A video that has not
yet completed a successful download SHALL have no recorded filename.

#### Scenario: Download succeeds
- **WHEN** a video download completes successfully
- **THEN** the video's recorded filename becomes the exact name of the file that was saved to disk

#### Scenario: Video not yet successfully downloaded
- **WHEN** a video is pending, in progress, or has only failed attempts so far
- **THEN** the video has no recorded filename

#### Scenario: Video re-downloaded after being reset
- **WHEN** a video that previously had a recorded filename is reset and successfully downloaded again
- **THEN** its recorded filename becomes the filename of the new download, replacing the prior one
