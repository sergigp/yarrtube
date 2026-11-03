# video-chapters Specification

## Purpose

Records each downloaded video's YouTube chapters (start, end and title), so
clients can offer chapter navigation, and embeds them into the downloaded
file for media servers.

## Requirements

### Requirement: Chapters Recorded On Download
The system SHALL record, on the video itself, the chapters YouTube reports for it, whenever a download succeeds: each chapter's start and end in whole seconds and its title, in playback order, and the download time as the chapters' last check. A video whose download succeeded without chapters SHALL be recorded as having no chapters. Chapters found SHALL be settled; no chapters SHALL be settled only when the video is old (see `late-video-data`). When the chapters could not be read during the download, the video's chapters SHALL be left unchecked.

#### Scenario: Download of a video with chapters
- **WHEN** a video download completes successfully and YouTube reports chapters "Game 1" (0–1037 s), "Game 2" (1037–2201 s) and "Game 3" (2201–3521 s) for it
- **THEN** the video's recorded chapters are exactly those three, in that order, and they are settled

#### Scenario: Download of a video without chapters
- **WHEN** a video published 30 days ago is downloaded successfully and YouTube reports no chapters for it
- **THEN** the video is recorded as having no chapters, settled, which is distinct from its chapters never having been checked

#### Scenario: Download of a young video without chapters
- **WHEN** a video published 1 hour ago is downloaded successfully and YouTube reports no chapters for it
- **THEN** the video is recorded as having no chapters, unsettled, last checked at the download time

#### Scenario: Chapters unreadable during the download
- **WHEN** a video download completes successfully but its chapters could not be read
- **THEN** the download is still recorded as successful and the video's chapters are left unchecked

#### Scenario: Library upgraded
- **WHEN** the system starts on a database with videos whose chapters were checked before chapters could be unsettled
- **THEN** those videos' chapters are settled, and videos whose chapters were never checked stay unchecked

### Requirement: Chapters Embedded In The Downloaded File
The system SHALL embed a video's chapters as chapter markers in the downloaded media file whenever YouTube reports chapters for it, so media servers that read embedded chapters can offer them.

#### Scenario: Downloaded video with chapters
- **WHEN** a video with chapters is downloaded
- **THEN** its media file contains one chapter marker per chapter, with the chapter's title and start time

### Requirement: Chapters Fetched For Unchecked Videos
The system SHALL fetch the chapters of a Downloaded video whose chapters are unsettled, without downloading the video again, and record them and the check time the same way a download does, settling them under the `late-video-data` policy. When the fetch fails, the system SHALL keep the chapters it already recorded (or leave them unchecked), count one failed attempt, settle them when the video is old and has at least 3 failed attempts (as having no chapters when none were recorded), and SHALL NOT change anything else about the video.

#### Scenario: Unchecked video with chapters on YouTube
- **WHEN** a chapters fetch runs for a Downloaded video whose chapters were never checked, and YouTube reports chapters for it
- **THEN** the video's recorded chapters become those chapters, settled, and its media file is not downloaded again

#### Scenario: Young video gains chapters later
- **WHEN** a chapters fetch runs for a video published 3 days ago recorded as having no chapters (unsettled), and YouTube now reports chapters for it
- **THEN** the video's recorded chapters become those chapters, settled

#### Scenario: Unchecked video without chapters on YouTube
- **WHEN** a chapters fetch runs for a Downloaded video published 30 days ago whose chapters were never checked, and YouTube reports no chapters for it
- **THEN** the video is recorded as having no chapters, settled

#### Scenario: Chapters fetch fails
- **WHEN** a chapters fetch runs for a young video and the chapters cannot be read (e.g. a network error)
- **THEN** the video's chapters stay as they were and unsettled, one failed attempt is counted, and none of its other recorded fields change

#### Scenario: Old video's third failed chapters fetch
- **WHEN** a chapters fetch fails for a video published 30 days ago, with unchecked chapters and 2 failed attempts
- **THEN** the video is recorded as having no chapters, settled

#### Scenario: Video already checked or gone
- **WHEN** a chapters fetch runs for a video whose chapters are settled, or whose record no longer exists
- **THEN** nothing is fetched and nothing is changed

### Requirement: Recording Chapters Changes Only Chapters
Recording a video's fetched chapters SHALL change only that video's recorded chapters and its last-updated time, never its download status or any other field, even if the video changed since the fetch started.

#### Scenario: Watch progress recorded during a chapters fetch
- **WHEN** a video's playback position is recorded while a chapters fetch for it is running
- **THEN** after the fetch records the chapters, the video keeps the newly recorded playback position
