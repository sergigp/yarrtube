# Spec Delta

## MODIFIED Requirements

### Requirement: Watch State Per YouTube Video
The system SHALL record, for each video, whether it has been watched and its saved playback position in whole seconds. A video that has never been played SHALL be unwatched with a saved position of 0. Watch state SHALL belong to the YouTube video: every change SHALL apply to every stored copy of that YouTube video, whichever playlist or channel tracks it. A download SHALL NOT undo a watch state recorded while it was in flight.

#### Scenario: A newly recorded video is unwatched
- **WHEN** a video is recorded for a playlist or channel for the first time
- **THEN** it is unwatched with a saved playback position of 0

#### Scenario: A change applies to every stored copy
- **WHEN** the same YouTube video is tracked by both a channel and a playlist, and its watch state changes
- **THEN** both stored copies report the same new watch state

#### Scenario: Watch state survives a redownload
- **WHEN** a watched video's file goes missing and the video is scheduled for a fresh download
- **THEN** the video stays watched

#### Scenario: Watch state survives a download in flight
- **WHEN** a video's watch state changes while its download is in progress, e.g. because another copy of it was marked watched, and the download then succeeds, fails or ends excluded
- **THEN** the video keeps the watch state recorded during the download
