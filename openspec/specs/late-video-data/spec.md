# late-video-data Specification

## Purpose

One policy for per-video YouTube data that can be missing or incomplete right after upload (captions, chapters, processed HD formats) but is final a few days later: keep checking young videos, settle old ones, and never loop forever on a failing check.

## Requirements

### Requirement: Young And Old Videos
A video SHALL count as young until 7 days have passed since its publish time, and as old from then on. When the publish time is not known, the video's record creation time SHALL be used instead. Each check SHALL use the publish time the downloader reports during that check when it reports one.

#### Scenario: Video published 2 days ago
- **WHEN** a check runs for a video published 2 days ago
- **THEN** the video counts as young

#### Scenario: Video published years ago
- **WHEN** a check runs for a video published in 2021, whose record was created yesterday (e.g. its playlist was added yesterday)
- **THEN** the video counts as old

#### Scenario: Publish time unknown
- **WHEN** a check fails before the downloader reports a publish time, for a video whose record was created 30 days ago
- **THEN** the video counts as old

### Requirement: Settling Late Data
Each piece of late data (subtitles, chapters, downloaded quality) SHALL be either settled (final, never checked again) or unsettled (checked again later). A conclusive check that finds the data complete SHALL settle it. A conclusive check that finds the data missing or incomplete SHALL settle it when the video is old, and SHALL leave it unsettled when the video is young. Each check SHALL record when it ran.

#### Scenario: Young video without the data yet
- **WHEN** a conclusive check finds no captions for a video published 2 days ago
- **THEN** its subtitles stay unsettled and the check time is recorded

#### Scenario: Old video without the data
- **WHEN** a conclusive check finds no captions for a video published 30 days ago, checked for the first time
- **THEN** its subtitles are settled at once, as unavailable

#### Scenario: Data found
- **WHEN** a conclusive check finds the data for a young video
- **THEN** that data is settled

### Requirement: Re-Check Spacing
Pending or Retrying data SHALL be checked again only when it is due: never checked; or it is Pending after a conclusive check (data missing or incomplete on a young video) that ran at least 24 hours earlier; or it is Retrying and its next attempt under the `video-retries` backoff is due (10 minutes after the 1st consecutive failure, 1 hour after the 2nd, 6 hours after the 3rd, 24 hours after each later one). A check scheduled right after a download finishes counts as the first check. A failed check SHALL re-queue itself for its next attempt.

#### Scenario: Checked 3 hours ago
- **WHEN** a reconcile pass runs for a video whose subtitles are Pending and were last checked 3 hours ago
- **THEN** no subtitle check is scheduled

#### Scenario: Checked 25 hours ago
- **WHEN** a reconcile pass runs for a video whose chapters are Pending and were last checked 25 hours ago
- **THEN** a chapters check is scheduled

#### Scenario: Settled data
- **WHEN** a reconcile pass runs for a video whose subtitles, chapters and quality are all Done, Unavailable or Failed
- **THEN** none of those checks is scheduled, however long ago they ran

#### Scenario: First failed check
- **WHEN** a chapters check fails at 10:00 for the first time
- **THEN** the chapters check is re-queued to run at 10:10

### Requirement: Redownload Starts Over
Resetting a video for redownload SHALL make its subtitles, chapters and downloaded quality unsettled again, with no check time and no failed attempts recorded.

#### Scenario: Broken file reset
- **WHEN** a video whose subtitles, chapters and quality are settled is reset for redownload
- **THEN** all three are unsettled, never checked, with no failed attempts

### Requirement: Failed Checks Follow The Retry Policy
A check that fails (the downloader errors) or is inconclusive (the downloader reports a warning that can affect the result) SHALL make the data Retrying and count one consecutive failed attempt; a conclusive check SHALL clear them. After the 11th consecutive failed attempt the data SHALL become Failed (see `video-retries`), whether the video is young or old, and is not checked again until the user retries the video. A conclusive check SHALL make the data Done when it finds it, Unavailable when it finds it missing on an old video, and leave it Pending when it finds it missing on a young video.

#### Scenario: Old video fails once
- **WHEN** a check fails for an old video whose data has no failed attempts
- **THEN** the data is Retrying with 1 failed attempt

#### Scenario: Young video keeps failing
- **WHEN** a check fails for a young video whose data already has 5 failed attempts
- **THEN** the data is Retrying with 6 failed attempts

#### Scenario: Eleventh failed check
- **WHEN** a check fails for a video whose data already has 10 consecutive failed attempts
- **THEN** the data is Failed and is not checked again until the user retries the video

#### Scenario: Conclusive check after failures
- **WHEN** a conclusive check finds no captions for a young video whose subtitles have 4 failed attempts
- **THEN** its subtitles are Pending with no failed attempts and are checked again 24 hours later
