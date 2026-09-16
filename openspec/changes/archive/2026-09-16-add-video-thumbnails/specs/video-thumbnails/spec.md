## Purpose

Captures a thumbnail image for every downloaded video — embedded directly in the video file and written as a matching sibling image file — so the video has visual cover art usable by media players, media servers, and a future video list UI, without any extra download step.

## ADDED Requirements

### Requirement: Thumbnail Embedded In Video File
The system SHALL embed the video's thumbnail image as cover art in the downloaded video file itself, whenever a thumbnail is available for that video.

#### Scenario: Video downloaded with a thumbnail available
- **WHEN** a video download succeeds and a thumbnail is available for it
- **THEN** the saved video file has that thumbnail embedded as its cover art

### Requirement: Thumbnail File Written Alongside Video
The system SHALL, whenever a thumbnail is available for a downloaded video, write it to the video's output directory as a `.jpg` file sharing the exact same base filename (everything before the extension) as the video's own saved file.

#### Scenario: Video downloaded with a thumbnail available
- **WHEN** a video download succeeds and a thumbnail is available for it
- **THEN** a `.jpg` file is written in the same output directory, named identically to the video file except for its extension

#### Scenario: Thumbnail unavailable for an otherwise successful download
- **WHEN** a video download succeeds but no thumbnail could be obtained for it
- **THEN** no thumbnail file is written, and the video download is not treated as failed
