# video-thumbnails Specification

## Purpose

Captures a thumbnail image for every downloaded video — embedded directly in the video file and written as a matching sibling image file — so the video has visual cover art usable by media players, media servers, and a future video list UI, without any extra download step.

## Requirements

### Requirement: Thumbnail Embedded In Video File
The system SHALL embed the video's thumbnail image as cover art in the downloaded video file itself, whenever a thumbnail is available for that video.

#### Scenario: Video downloaded with a thumbnail available
- **WHEN** a video download succeeds and a thumbnail is available for it
- **THEN** the saved video file has that thumbnail embedded as its cover art

### Requirement: Thumbnail File Written Alongside Video
The system SHALL, whenever a thumbnail is available for a downloaded video, write it to the video's own output folder as a `.jpg` file sharing the exact same base filename (everything before the extension) as the video's own saved file.

#### Scenario: Video downloaded with a thumbnail available
- **WHEN** a video download succeeds and a thumbnail is available for it
- **THEN** a `.jpg` file is written in that video's own output folder, named identically to the video file except for its extension

#### Scenario: Thumbnail unavailable for an otherwise successful download
- **WHEN** a video download succeeds but no thumbnail could be obtained for it
- **THEN** no thumbnail file is written, and the video download is not treated as failed

### Requirement: Thumbnail Fetched Ahead Of Video Download
The system SHALL fetch a video's thumbnail image independently of its full video download, as a separately scheduled task, so a thumbnail can be available while the video is still pending or in progress. The thumbnail fetch SHALL be scheduled as soon as the video is newly added to a tracked playlist or channel, in reaction to that addition, and SHALL NOT be performed while the video's record is being created. The fetch SHALL be best-effort:
- a failure SHALL NOT be treated as a download error,
- a failure SHALL NOT change the video's status,
- a failure SHALL NOT prevent the video's full download from being scheduled or run.

A thumbnail fetch that runs after the video already has a recorded thumbnail filename SHALL make no changes. A thumbnail fetch that runs while the video's download status is Excluded or Errored SHALL make no changes and SHALL NOT attempt to obtain the thumbnail; this covers a fetch scheduled before the video reached that status.

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

#### Scenario: Fetch runs for an excluded video
- **WHEN** a thumbnail fetch was scheduled for a video (e.g. on its addition) and, by the time it runs, the video's download status is Excluded
- **THEN** the fetch does not try to obtain a thumbnail and makes no changes to the video or to disk

#### Scenario: Fetch runs for an errored video
- **WHEN** a thumbnail fetch runs for a video whose download status is Errored
- **THEN** the fetch does not try to obtain a thumbnail and makes no changes to the video or to disk

### Requirement: Thumbnail Recording Preserves Download Progress
Recording a fetched thumbnail's filename on a video SHALL change only that video's recorded thumbnail filename (and its last-updated time), never its download status or any field set by its download.

#### Scenario: Download progresses while a thumbnail is recorded
- **WHEN** a video's status changes (e.g. its download starts or completes) between the time a thumbnail fetch reads the video and the time it records the thumbnail
- **THEN** the video keeps its new status and download fields, and gains the recorded thumbnail filename
