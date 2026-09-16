## ADDED Requirements

### Requirement: Downloaded Thumbnail Is Recorded
The system SHALL record, on the video itself, the exact filename of the
thumbnail file that was saved to disk alongside it, whenever a download
succeeds and a thumbnail file was written for it. A video that has not yet
completed a successful download, or whose download succeeded without a
thumbnail being written, SHALL have no recorded thumbnail filename.

#### Scenario: Download succeeds with a thumbnail
- **WHEN** a video download completes successfully and a thumbnail file was written for it
- **THEN** the video's recorded thumbnail filename becomes the exact name of that thumbnail file

#### Scenario: Download succeeds without a thumbnail
- **WHEN** a video download completes successfully but no thumbnail file was written for it
- **THEN** the video has no recorded thumbnail filename

#### Scenario: Video not yet successfully downloaded
- **WHEN** a video is pending, in progress, or has only failed attempts so far
- **THEN** the video has no recorded thumbnail filename

#### Scenario: Video re-downloaded after being reset
- **WHEN** a video that previously had a recorded thumbnail filename is reset and successfully downloaded again
- **THEN** its recorded thumbnail filename reflects the new download (a fresh filename, or none, replacing the prior one)
