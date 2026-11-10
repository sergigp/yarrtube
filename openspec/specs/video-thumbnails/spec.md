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
The system SHALL, whenever a thumbnail is available for a downloaded video, write it to the video's season folder as a `.jpg` file sharing the exact same episode base name (everything before the extension) as the video's file, so the thumbnail sits next to the video and media players pick it up as the episode's image.

#### Scenario: Video downloaded with a thumbnail available
- **WHEN** a video download succeeds and a thumbnail is available for it
- **THEN** a `.jpg` file is written in that video's season folder, named identically to the video file except for its extension

#### Scenario: Thumbnail unavailable for an otherwise successful download
- **WHEN** a video download succeeds but no thumbnail could be obtained for it
- **THEN** no thumbnail file is written, and the video download is not treated as failed

### Requirement: Thumbnail Fetched Ahead Of Video Download
The system SHALL fetch a video's thumbnail image independently of its full video download, as a separately scheduled task, so a thumbnail can be available while the video is still pending or in progress. The fetch SHALL use the video's recorded episode number, assigning a channel video's if it has none yet (from the publish timestamp the downloader reports with the thumbnail, see `tv-show-layout`), and SHALL write the thumbnail at the episode base name in the season folder, where the later download reuses it. The thumbnail fetch is best effort:
- a failure SHALL NOT be treated as a download error,
- a failure SHALL NOT change the video's status,
- a failure SHALL NOT prevent the video's full download from being scheduled or run,
- a fetch that obtains no thumbnail SHALL make the thumbnail Retrying, count one failed attempt and re-queue itself under the `video-retries` policy; after the 11th consecutive one the thumbnail is Failed and is no longer fetched until the user retries the video. A recorded thumbnail, by the fetch or by the download, makes it Done.

A thumbnail fetch that runs after the video already has a recorded thumbnail filename SHALL make no changes. A thumbnail fetch that runs while the video's download is Unavailable, Retrying or Failed SHALL make no changes and SHALL NOT try to obtain a thumbnail.

#### Scenario: Video created and a thumbnail is available
- **WHEN** a video is newly added to a tracked playlist or channel and its thumbnail can be obtained
- **THEN** a thumbnail fetch is scheduled for it, and when that fetch runs the system records the video's episode number when it had none and the thumbnail's filename (`Season <season>/<episode base>.jpg`) on the video

#### Scenario: Video created and no thumbnail is available yet
- **WHEN** a video is newly added and its scheduled thumbnail fetch cannot obtain a thumbnail (e.g. a transient failure)
- **THEN** the video is left with no recorded thumbnail filename, its status is unchanged, its thumbnail is Retrying with one failed attempt, the fetch is re-queued for 10 minutes later, and its download is still scheduled and run

#### Scenario: Video creation does not wait for the thumbnail
- **WHEN** a reconcile pass creates a new video's record
- **THEN** the pass does not wait for that video's thumbnail to be fetched before persisting the next video or publishing the video's addition

#### Scenario: Download already recorded a thumbnail
- **WHEN** a video's thumbnail fetch runs after that video's download already recorded a thumbnail filename
- **THEN** the fetch makes no changes to the video or to disk

#### Scenario: Download without a thumbnail keeps the fetched one
- **WHEN** a video's thumbnail was fetched ahead of its download and the download brings no thumbnail
- **THEN** the video keeps its recorded thumbnail filename, its thumbnail stays Done and its episode NFO references it

#### Scenario: Fetch runs for an excluded video
- **WHEN** a thumbnail fetch was scheduled for a video (e.g. on its addition) and, by the time it runs, the video's download is Unavailable
- **THEN** the fetch does not try to obtain a thumbnail and makes no changes to the video or to disk

#### Scenario: Fetch runs for an errored video
- **WHEN** a thumbnail fetch runs for a video whose download is Retrying or Failed
- **THEN** the fetch does not try to obtain a thumbnail and makes no changes to the video or to disk

#### Scenario: Thumbnail gives up
- **WHEN** a thumbnail fetch obtains no thumbnail for a video with 10 consecutive failed thumbnail attempts
- **THEN** the thumbnail is Failed and no thumbnail fetch is scheduled for the video again

### Requirement: Thumbnail Recording Preserves Download Progress
Recording a fetched thumbnail's filename on a video SHALL change only that video's recorded thumbnail filename, its episode number when it had none, and its last-updated time, never its download status or any field set by its download.

#### Scenario: Download progresses while a thumbnail is recorded
- **WHEN** a video's status changes (e.g. its download starts or completes) between the time a thumbnail fetch reads the video and the time it records the thumbnail
- **THEN** the video keeps its new status and download fields, and gains the recorded thumbnail filename
