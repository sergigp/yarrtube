## ADDED Requirements

### Requirement: Chapters Checked For Unsettled Videos
The system SHALL fetch the chapters of a Downloaded video whose chapters are Pending or Retrying (and due), without downloading the video again, and record them and the check time the same way a download does, making them Done when found, Unavailable when an old video has none, and leaving them Pending when a young video has none (see `late-video-data`). When the fetch fails, the system SHALL keep the chapters it already recorded (or leave them unchecked), count one failed attempt, make them Retrying and re-queue the fetch under the `video-retries` policy, make them Failed after the 11th consecutive failed attempt, and SHALL NOT change anything else about the video.

#### Scenario: Unchecked video with chapters on YouTube
- **WHEN** a chapters fetch runs for a Downloaded video whose chapters were never checked, and YouTube reports chapters for it
- **THEN** the video's recorded chapters become those chapters, Done, and its media file is not downloaded again

#### Scenario: Young video gains chapters later
- **WHEN** a chapters fetch runs for a video published 3 days ago recorded as having no chapters (Pending), and YouTube now reports chapters for it
- **THEN** the video's recorded chapters become those chapters, Done

#### Scenario: Unchecked video without chapters on YouTube
- **WHEN** a chapters fetch runs for a Downloaded video published 30 days ago whose chapters were never checked, and YouTube reports no chapters for it
- **THEN** the video is recorded as having no chapters, Unavailable

#### Scenario: Chapters fetch fails
- **WHEN** a chapters fetch runs for a young video and the chapters cannot be read (e.g. a network error)
- **THEN** the video's chapters stay as they were and are Retrying, one failed attempt is counted, the fetch is re-queued for its next attempt, and none of its other recorded fields change

#### Scenario: Eleventh failed chapters fetch
- **WHEN** a chapters fetch fails for a video with unchecked chapters and 10 consecutive failed attempts
- **THEN** the video's chapters are Failed

#### Scenario: Video already checked or gone
- **WHEN** a chapters fetch runs for a video whose chapters are Done, Unavailable or Failed, or whose record no longer exists
- **THEN** nothing is fetched and nothing is changed

## REMOVED Requirements

### Requirement: Chapters Fetched For Unchecked Videos
**Reason**: Replaced by "Chapters Checked For Unsettled Videos": failed fetches follow the `video-retries` policy instead of settling an old video after 3 failures.
**Migration**: See `late-video-data` "Failed Checks Follow The Retry Policy".
