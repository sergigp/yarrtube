## MODIFIED Requirements

### Requirement: Thumbnail Fetched Ahead Of Video Download
The system SHALL fetch a video's thumbnail image independently of its full video download, as a separately scheduled task, so a thumbnail can be available while the video is still pending or in progress. The thumbnail fetch SHALL be scheduled as soon as the video is newly added to a tracked playlist or channel, in reaction to that addition, and SHALL NOT be performed while the video's record is being created. The fetch SHALL be best-effort:
- a failure SHALL NOT be treated as a download error,
- a failure SHALL NOT change the video's status,
- a failure SHALL NOT prevent the video's full download from being scheduled or run.

A thumbnail fetch that runs after the video already has a recorded thumbnail filename SHALL make no changes.

#### Scenario: Video created and a thumbnail is available
- **WHEN** a video is newly added to a tracked playlist or channel and its thumbnail can be obtained
- **THEN** a thumbnail fetch is scheduled for it, and when that fetch runs the system records the thumbnail's filename on the video

#### Scenario: Video created and no thumbnail is available yet
- **WHEN** a video is newly added and its scheduled thumbnail fetch cannot obtain a thumbnail (e.g. a transient failure)
- **THEN** the video is left with no recorded thumbnail filename, its status is unchanged, and its download is still scheduled and run

#### Scenario: Video creation does not wait for the thumbnail
- **WHEN** a reconcile pass creates a new video's record
- **THEN** the pass does not wait for that video's thumbnail to be fetched before persisting the next video or publishing the video's addition

#### Scenario: Download already recorded a thumbnail
- **WHEN** a video's thumbnail fetch runs after that video's download already recorded a thumbnail filename
- **THEN** the fetch makes no changes to the video or to disk

## ADDED Requirements

### Requirement: Thumbnail Recording Preserves Download Progress
Recording a fetched thumbnail's filename on a video SHALL change only that video's recorded thumbnail filename (and its last-updated time), never its download status or any field set by its download.

#### Scenario: Download progresses while a thumbnail is recorded
- **WHEN** a video's status changes (e.g. its download starts or completes) between the time a thumbnail fetch reads the video and the time it records the thumbnail
- **THEN** the video keeps its new status and download fields, and gains the recorded thumbnail filename
