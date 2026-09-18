## MODIFIED Requirements

### Requirement: Thumbnail File Written Alongside Video
The system SHALL, whenever a thumbnail is available for a downloaded video, write it to the video's own output folder as a `.jpg` file sharing the exact same base filename (everything before the extension) as the video's own saved file.

#### Scenario: Video downloaded with a thumbnail available
- **WHEN** a video download succeeds and a thumbnail is available for it
- **THEN** a `.jpg` file is written in that video's own output folder, named identically to the video file except for its extension

#### Scenario: Thumbnail unavailable for an otherwise successful download
- **WHEN** a video download succeeds but no thumbnail could be obtained for it
- **THEN** no thumbnail file is written, and the video download is not treated as failed
