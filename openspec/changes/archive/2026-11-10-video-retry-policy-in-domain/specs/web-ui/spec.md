## MODIFIED Requirements

### Requirement: Video Detail Status And Location
The video detail pane SHALL always show, below the meta line, the video's storage location and, when the video is not downloaded, a status indicator, at every viewport size and with no control to expand or collapse them. The storage location SHALL be the folder holding the video's file, never the file name itself, so episode codes do not appear in the UI. The download status indicator SHALL read "Pending", "Downloading", "Retrying", "Failed" or "Unavailable". The pane SHALL list every step of the video that is Retrying or Failed, each with its name, its status, and for a Retrying step when its next attempt is due, relative to now (e.g. "next attempt in 2 hours"), or for a Failed step "gave up after 11 attempts", followed by the last failure reason where one is recorded. The pane SHALL NOT show a quality indicator, a status indicator for a downloaded video, or the video's YouTube description.

#### Scenario: Details on a mobile viewport
- **WHEN** a user on a mobile-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the video's title, its meta line and its storage location, with no "More" or "Less" control

#### Scenario: Details on a desktop viewport
- **WHEN** a user on a desktop-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the video's title, its meta line and its storage location, with no "More" or "Less" control

#### Scenario: Storage location of a downloaded episode
- **WHEN** a downloaded video's recorded filename is `Season 2026/S2026E031501 - Title.mp4` in a channel stored at `channels/some-channel`
- **THEN** the detail pane shows `channels/some-channel/Season 2026` and not the file name

#### Scenario: Storage location of a video not yet downloaded
- **WHEN** a video has no recorded filename
- **THEN** the detail pane shows its source's storage path

#### Scenario: Downloaded video shows no status indicator
- **WHEN** a user selects a downloaded video
- **THEN** no status or quality indicator is shown

#### Scenario: Pending video shows its status
- **WHEN** a user selects a video that is pending, downloading, retrying, failed or unavailable
- **THEN** a status indicator shows that state, and no quality indicator is shown

#### Scenario: Retrying download
- **WHEN** a user selects a video whose download failed with "HTTP Error 503" and whose next attempt is due in 2 hours
- **THEN** the status indicator reads "Retrying" and the pane lists "Download: retrying, next attempt in 2 hours · HTTP Error 503"

#### Scenario: Failed download
- **WHEN** a user selects a video whose download gave up with "HTTP Error 503"
- **THEN** the status indicator reads "Failed" and the pane lists "Download: failed, gave up after 11 attempts · HTTP Error 503"

#### Scenario: Downloaded video with failing steps
- **WHEN** a user selects a downloaded video whose subtitles are Retrying with the next attempt in 1 hour and whose summary is Failed
- **THEN** no status indicator is shown and the pane lists "Subtitles: retrying, next attempt in 1 hour" and "Summary: failed, gave up after 11 attempts"

#### Scenario: Unavailable video
- **WHEN** a user selects a video whose download is Unavailable
- **THEN** the status indicator reads "Unavailable"

## ADDED Requirements

### Requirement: Retry A Video From The Video Detail
The video detail pane SHALL show a "Retry now" button when any step of the selected video is Retrying or Failed. Activating it SHALL retry the video (see `video-retries`) and refresh the video, without a page reload. When the retry fails, the application SHALL tell the user and leave the video as it was. The button SHALL NOT be shown when no step is Retrying or Failed.

#### Scenario: Retrying a failed download
- **WHEN** a user selects a video whose download is Failed and activates "Retry now"
- **THEN** its status indicator reads "Pending" or "Downloading", and the button is gone

#### Scenario: Retrying a download waiting for its next attempt
- **WHEN** a user selects a video whose download is Retrying with its next attempt in 1 hour and activates "Retry now"
- **THEN** its download runs again without waiting for the next attempt

#### Scenario: Retrying a failed summary
- **WHEN** a user selects a downloaded video whose summary is Failed
- **THEN** a "Retry now" button is shown

#### Scenario: Nothing to retry
- **WHEN** a user selects a downloaded video with no Retrying or Failed step
- **THEN** no "Retry now" button is shown

#### Scenario: Retry fails
- **WHEN** a user activates "Retry now" and the request fails
- **THEN** the application tells the user and the video is shown as before
