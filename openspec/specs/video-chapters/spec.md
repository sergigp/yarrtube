# video-chapters Specification

## Purpose

Records each downloaded video's YouTube chapters (start, end and title), so
clients can offer chapter navigation, and embeds them into the downloaded
file for media servers.

## Requirements

### Requirement: Chapters Recorded On Download
The system SHALL record, on the video itself, the chapters YouTube reports for it, whenever a download succeeds: each chapter's start and end in whole seconds and its title, in playback order. A video whose download succeeded without chapters SHALL be recorded as having no chapters. When the chapters could not be read during the download, the video's chapters SHALL be left unchecked.

#### Scenario: Download of a video with chapters
- **WHEN** a video download completes successfully and YouTube reports chapters "Game 1" (0–1037 s), "Game 2" (1037–2201 s) and "Game 3" (2201–3521 s) for it
- **THEN** the video's recorded chapters are exactly those three, in that order

#### Scenario: Download of a video without chapters
- **WHEN** a video download completes successfully and YouTube reports no chapters for it
- **THEN** the video is recorded as having no chapters, which is distinct from its chapters never having been checked

#### Scenario: Chapters unreadable during the download
- **WHEN** a video download completes successfully but its chapters could not be read
- **THEN** the download is still recorded as successful and the video's chapters are left unchecked

### Requirement: Chapters Embedded In The Downloaded File
The system SHALL embed a video's chapters as chapter markers in the downloaded media file whenever YouTube reports chapters for it, so media servers that read embedded chapters can offer them.

#### Scenario: Downloaded video with chapters
- **WHEN** a video with chapters is downloaded
- **THEN** its media file contains one chapter marker per chapter, with the chapter's title and start time

### Requirement: Chapters Fetched For Unchecked Videos
The system SHALL fetch the chapters of a Downloaded video whose chapters were never checked, without downloading the video again, and record them the same way a download does. When the fetch fails, the system SHALL leave the video's chapters unchecked, so a later video sync tries again, and SHALL NOT change anything else about the video.

#### Scenario: Unchecked video with chapters on YouTube
- **WHEN** a chapters fetch runs for a Downloaded video whose chapters were never checked, and YouTube reports chapters for it
- **THEN** the video's recorded chapters become those chapters and its media file is not downloaded again

#### Scenario: Unchecked video without chapters on YouTube
- **WHEN** a chapters fetch runs for a Downloaded video whose chapters were never checked, and YouTube reports no chapters for it
- **THEN** the video is recorded as having no chapters

#### Scenario: Chapters fetch fails
- **WHEN** a chapters fetch runs and the chapters cannot be read (e.g. the video is no longer available on YouTube)
- **THEN** the video's chapters stay unchecked and none of its other recorded fields change

#### Scenario: Video already checked or gone
- **WHEN** a chapters fetch runs for a video whose chapters were already checked, or whose record no longer exists
- **THEN** nothing is fetched and nothing is changed

### Requirement: Recording Chapters Changes Only Chapters
Recording a video's fetched chapters SHALL change only that video's recorded chapters and its last-updated time, never its download status or any other field, even if the video changed since the fetch started.

#### Scenario: Watch progress recorded during a chapters fetch
- **WHEN** a video's playback position is recorded while a chapters fetch for it is running
- **THEN** after the fetch records the chapters, the video keeps the newly recorded playback position
