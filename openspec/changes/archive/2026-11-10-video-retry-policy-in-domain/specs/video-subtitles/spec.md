## MODIFIED Requirements

### Requirement: Subtitle State
Every video SHALL record a subtitle status from the `video-retries` vocabulary: Pending (formerly Missing: no subtitles obtained yet), Retrying (the last checks failed or were inconclusive), Done (formerly Fetched: subtitle file and transcript recorded), Unavailable (no captions on YouTube, settled under the `late-video-data` policy) or Failed (gave up after 11 failed checks), stored as `PENDING`, `RETRYING`, `DONE`, `UNAVAILABLE` and `FAILED`. A video SHALL also record when its subtitles were last checked and how many consecutive checks failed or were inconclusive. A new video SHALL start Pending, never checked, with no failed attempts. A video stored before this policy with a recorded first check without captions SHALL keep that time as its last check, with no failed attempts. A video reset for redownload SHALL return to Pending, with no subtitle filename, never checked and no failed attempts.

#### Scenario: New video
- **WHEN** a video is newly added to a tracked playlist or channel
- **THEN** its subtitles are Pending, it has no recorded subtitle filename, and its subtitles were never checked

#### Scenario: Library upgraded
- **WHEN** the system starts on a database where a video with Pending subtitles recorded its first check without captions at a given time
- **THEN** that video's subtitles are recorded as last checked at that time, with no failed attempts

#### Scenario: Video reset for redownload
- **WHEN** a video whose subtitles are Done, Unavailable or Failed is reset for redownload
- **THEN** its subtitles are Pending, it has no recorded subtitle filename, its subtitles were never checked, and it has no failed attempts

#### Scenario: Stored states upgraded
- **WHEN** the system starts on a database with subtitle states `MISSING`, `FETCHED` and `UNAVAILABLE`
- **THEN** they become `PENDING` (or `RETRYING` when failed attempts are recorded), `DONE`, and `UNAVAILABLE` (or `FAILED` when settled after at least 3 failed attempts)

### Requirement: Subtitle Fetch Is Best Effort
A subtitle fetch SHALL run as its own scheduled task, separately from the video download. Its failure SHALL NOT fail the task and SHALL NOT change the video's download status or any download field; the fetch is re-queued under the `video-retries` policy instead. A subtitle fetch for a video that is not Downloaded, has no episode number, is still stored in the previous per-video-folder layout (it gets subtitles once `migrate-layout` has moved it into its season folder), or whose subtitles are neither Pending nor Retrying SHALL make no changes and SHALL NOT contact the downloader. A fetch SHALL place nothing when, by the time its subtitles are downloaded, the video is no longer Downloaded with the same recorded filename.

#### Scenario: Fetch runs for a video already Fetched
- **WHEN** a subtitle fetch runs for a video whose subtitles are already Done, Unavailable or Failed
- **THEN** it makes no changes to the video or to disk

#### Scenario: Fetch runs for a video no longer downloaded
- **WHEN** a subtitle fetch runs for a video that has meanwhile been reset for redownload
- **THEN** it makes no changes and does not contact the downloader

#### Scenario: Fetch runs for a deleted video
- **WHEN** a subtitle fetch runs after its video's record was deleted
- **THEN** it completes without changes

#### Scenario: Video still in the previous layout
- **WHEN** a subtitle fetch runs for a Downloaded, numbered video whose recorded filename is still `My Video/My Video.mp4` or `My Video.mp4` (not yet migrated)
- **THEN** it makes no changes, writes no file and does not contact the downloader, and reconcile passes do not schedule a subtitle fetch for it

#### Scenario: Video reset for redownload while its subtitles are fetched
- **WHEN** a video is reset for redownload after a subtitle fetch for it has started but before it places the subtitle file
- **THEN** no subtitle file is placed, no transcript is stored, no subtitles-fetched event is published, and its subtitles stay as they were

#### Scenario: Video deleted while its subtitles are fetched
- **WHEN** a video's record is deleted (or its playlist or channel deleted) after a subtitle fetch for it has started but before it places the subtitle file
- **THEN** no subtitle file is placed, no transcript is stored and no subtitles-fetched event is published

#### Scenario: Recording subtitles preserves download progress
- **WHEN** a video's download fields change between the time a subtitle fetch reads the video and the time it records the subtitles
- **THEN** the video keeps its new download fields and gains the recorded subtitle status and filename

## ADDED Requirements

### Requirement: Subtitle Check Outcome
A subtitle fetch SHALL make the video's subtitles Done when it writes the subtitle file and stores the transcript. A check is conclusive when the downloader successfully reports the video's caption tracks without reporting any warning that can affect them (it can silently withhold tracks, e.g. when YouTube requires a token it does not have, and only says so in a warning). Warnings that cannot affect caption tracks SHALL be ignored: the downloader failing to write its cache, and notices that some video formats were skipped. Every fetch that contacts the downloader SHALL record its time as the subtitles' last check. When a conclusive check finds no selectable track, the subtitles SHALL become Unavailable when the video is old and stay Pending when it is young (see `late-video-data`). When the check is inconclusive or the downloader fails, the fetch SHALL count one failed attempt, make the subtitles Retrying and re-queue itself under the `video-retries` policy; after the 11th consecutive failed attempt the subtitles SHALL become Failed.

#### Scenario: Captions fetched
- **WHEN** a subtitle fetch selects a track and writes it
- **THEN** the video's subtitles are Done

#### Scenario: Old video with no original-language captions
- **WHEN** a conclusive check finds no selectable track for a video published 30 days ago whose subtitles were last checked 2 days ago
- **THEN** the video's subtitles are Unavailable and no subtitle file is written

#### Scenario: Old video checked for the first time
- **WHEN** a conclusive check finds no selectable track for a video published 30 days ago (e.g. of a playlist added today) that was never checked
- **THEN** its subtitles are Unavailable at once and no subtitle file is written

#### Scenario: Recent video with no captions yet
- **WHEN** a conclusive check finds no selectable track for a video published 2 days ago
- **THEN** its subtitles stay Pending and the check time is recorded, so a check at least 24 hours later tries again

#### Scenario: Downloader warns while checking
- **WHEN** the downloader reports the caption tracks with a warning that can affect them (e.g. `Some subtitles may be missing`), none is selectable, the video was published 30 days ago and has no failed attempts
- **THEN** its subtitles are Retrying with 1 failed attempt, and the warning is logged

#### Scenario: Eleventh failed check
- **WHEN** the downloader fails while probing captions for a video whose subtitles have 10 consecutive failed attempts
- **THEN** its subtitles are Failed

#### Scenario: Downloader only warns about its cache or skipped formats
- **WHEN** the downloader reports the caption tracks with only a cache write failure warning and a `formats have been skipped` warning, and none is selectable
- **THEN** the check is conclusive: no failed attempt is counted and nothing is logged as inconclusive

#### Scenario: Downloader fails
- **WHEN** the downloader fails while probing or downloading captions (e.g. rate limited, network error) for a video published 2 days ago
- **THEN** the video's subtitles are Retrying, one failed attempt is counted, no subtitle file or transcript is recorded, and the fetch is re-queued for its next attempt

## REMOVED Requirements

### Requirement: Subtitle Fetch Outcome
**Reason**: Replaced by "Subtitle Check Outcome": failed checks follow the `video-retries` policy instead of settling an old video after 3 failures.
**Migration**: See `late-video-data` "Failed Checks Follow The Retry Policy".
