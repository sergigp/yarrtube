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
The system SHALL attempt to fetch a video's thumbnail image independently of, and before, its full video download, as soon as that video's record is created, so a thumbnail can be available while the video is still pending or in progress. This fetch SHALL be best-effort: a failure SHALL NOT block the video's record from being created, SHALL NOT be treated as a download error, and SHALL NOT prevent the video's full download from being scheduled.

#### Scenario: Video created and a thumbnail is available
- **WHEN** a video's record is created and a thumbnail can be obtained for it
- **THEN** the system records that thumbnail's filename on the video before the video's full download runs

#### Scenario: Video created and no thumbnail is available yet
- **WHEN** a video's record is created but a thumbnail cannot be obtained for it (e.g. a transient failure)
- **THEN** the video's record is still created with no recorded thumbnail filename, and no error is surfaced to the caller that triggered its creation
