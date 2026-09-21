## ADDED Requirements

### Requirement: Thumbnail Fetched Ahead Of Video Download
The system SHALL attempt to fetch a video's thumbnail image independently of, and before, its full video download, as soon as that video's record is created, so a thumbnail can be available while the video is still pending or in progress. This fetch SHALL be best-effort: a failure SHALL NOT block the video's record from being created, SHALL NOT be treated as a download error, and SHALL NOT prevent the video's full download from being scheduled.

#### Scenario: Video created and a thumbnail is available
- **WHEN** a video's record is created and a thumbnail can be obtained for it
- **THEN** the system records that thumbnail's filename on the video before the video's full download runs

#### Scenario: Video created and no thumbnail is available yet
- **WHEN** a video's record is created but a thumbnail cannot be obtained for it (e.g. a transient failure)
- **THEN** the video's record is still created with no recorded thumbnail filename, and no error is surfaced to the caller that triggered its creation
