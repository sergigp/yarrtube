## MODIFIED Requirements

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

## ADDED Requirements

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

## REMOVED Requirements

### Requirement: Bounded Failed Checks
**Reason**: Replaced by "Failed Checks Follow The Retry Policy": failures back off and give up after 11 attempts, for young and old videos alike, instead of settling an old video after 3 failures and never settling a young one.
**Migration**: The stored failed attempts were never reset on success, so settled data is mapped by what was found. Found subtitles become Done; unavailable subtitles with at least 3 failed attempts become Failed (11 failed attempts), so the user can retry them, else Unavailable. Found chapters become Done; settled chapters with none found become Unavailable below 3 failed attempts and Failed (11) from 3. Settled quality becomes Done. Unsettled data with failed attempts becomes Retrying with them, else Pending.
