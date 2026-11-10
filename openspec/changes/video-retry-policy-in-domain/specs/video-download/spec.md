## MODIFIED Requirements

### Requirement: Video Status Reflects Download Progress
The system SHALL track a video's download status with the `video-retries` vocabulary: Pending, In Progress, Downloaded (Done), Retrying, Failed or Unavailable, stored as `PENDING`, `IN_PROGRESS`, `DOWNLOADED`, `RETRYING`, `FAILED` and `UNAVAILABLE`. A Retrying download failed and will be attempted again; the video SHALL record its failed attempts, when the last one failed and the last failure reason. A Failed download gave up after its last allowed attempt and is attempted again only when the user retries the video. A download becomes Unavailable when an attempt fails for a reason that can never succeed on a later attempt (see "Permanently Unavailable Videos Are Excluded"); Unavailable is final and is never retried, not even by the user's retry.

#### Scenario: Download starts
- **WHEN** the system begins downloading a Pending or Retrying video
- **THEN** the video's status becomes In Progress

#### Scenario: Download succeeds
- **WHEN** a video download completes successfully
- **THEN** the video's status becomes Downloaded and its failed attempts and failure reason are cleared

#### Scenario: Download fails with attempts remaining
- **WHEN** a video download attempt fails for a reason that is not permanently unavailable, and it is not the 11th consecutive failure
- **THEN** the video's status becomes Retrying, its failed attempts grow by one, and its failure time and reason are recorded

#### Scenario: Download fails permanently
- **WHEN** a video's download fails for the 11th consecutive time
- **THEN** the video's status becomes Failed

#### Scenario: Download fails for a diagnosed permanent reason
- **WHEN** a video download attempt fails and the diagnosed reason (see "Failed Downloads Are Diagnosed") is one that can never succeed later — members-only content, a copyright/claimed-content block, a region block, a private video, a removed video, or a terminated account
- **THEN** the video's status becomes Unavailable, however many attempts failed before

### Requirement: Download Failure Reason Is Recorded
The system SHALL record `yt-dlp`'s actual reported error text as a failed download attempt's failure reason on the video, when `yt-dlp` reported one, instead of a generic message that does not distinguish one failure cause from another. When the download could not run at all (for example `yt-dlp` could not be started or its output could not be read) or its files could not be placed, that error SHALL be recorded as the reason and the attempt SHALL count as a failed attempt.

#### Scenario: yt-dlp reports a specific error
- **WHEN** a video download attempt fails and `yt-dlp` reported an error message on its standard error stream
- **THEN** that exact message is recorded as the video's failure reason

#### Scenario: yt-dlp fails without reporting a message
- **WHEN** a video download attempt fails and `yt-dlp` reported no error message
- **THEN** a generic failure reason is recorded instead

#### Scenario: Downloaded files cannot be placed
- **WHEN** a download succeeds but moving its file into the show folder fails
- **THEN** the video is Retrying with that error as its reason and its next attempt is scheduled like any other failure

### Requirement: Permanently Unavailable Videos Are Excluded
The system SHALL mark a video's download Unavailable (formerly "excluded"), instead of Retrying, when the diagnosed reason for its failed download (see "Failed Downloads Are Diagnosed") is one that can never succeed on a later attempt. The reasons the system SHALL treat as permanently unavailable are: members-only content (requiring a paid channel membership `yt-dlp` cannot satisfy), a copyright or claimed-content block, a region/country block, a private video, a removed or no-longer-available video, and a video whose associated account has been terminated. An Unavailable video's download attempt SHALL be treated as a settled, non-failing outcome: the download task SHALL complete without error so the task queue neither retries nor dead-letters it. A failure whose diagnosed reason is NOT one of these — including an undetermined or merely generic reason (such as a bare "Video unavailable" with no further detail), or a network, timeout, HTTP server, rate-limit, bot-verification, or fragment/merge error — SHALL NOT be treated as permanently unavailable, and SHALL be retried under the `video-retries` policy.

#### Scenario: A diagnosed permanent reason excludes the video
- **WHEN** a video download attempt fails and the diagnosed reason is one of the permanently-unavailable reasons (members-only, copyright/claimed-content block, region block, private, removed, or terminated account)
- **THEN** the system marks the video Unavailable and completes the download task without a failure, so no retry is scheduled and the task is not dead-lettered

#### Scenario: A generic "Video unavailable" with no diagnosed cause is not excluded
- **WHEN** a video download attempt fails and the diagnosed reason is undetermined or merely a generic "Video unavailable" with no specific permanent cause
- **THEN** the system marks the video Retrying and schedules its next attempt, and does not make it Unavailable

#### Scenario: A recoverable failure is not excluded
- **WHEN** a video download attempt fails for a transient reason such as a network, timeout, HTTP server, rate-limit, bot-verification, or fragment error
- **THEN** the system marks the video Retrying and schedules its next attempt, and does not make it Unavailable

### Requirement: Young Video Quality Upgrade
The system SHALL run a quality check for a Downloaded video whose quality is Pending or Retrying: it SHALL ask the downloader which height the video's owning playlist's or channel's tier would download now, without downloading. When that height is above the recorded height, the system SHALL download the video again at that tier and replace its media file in place: same episode file name, same episode number, keeping its thumbnail, subtitles, chapters, metadata and watch state, and recording the new height and tier. The replaced file SHALL count as downloaded again for media-server scanning. Each quality check SHALL record its time as the quality's last check and make the quality Done when the new height reaches the tier's settlement height or the video is old, Pending otherwise. The media file SHALL be replaced only when the re-download's height is above the recorded one; otherwise the re-download SHALL be discarded and counted as a failed attempt. A probe or re-download the downloader reports as degraded by the SABR-only streaming experiment SHALL count as a failed attempt rather than a conclusive result; a degraded re-download that is still taller SHALL replace the file. A failed check or failed re-download SHALL keep the existing file and recorded fields unchanged, count one failed attempt, and re-queue the check under the `video-retries` policy without failing the task. A quality check SHALL make no changes for a video that is not Downloaded, whose quality is settled, or whose record, or owning playlist or channel, no longer exists.

#### Scenario: HD available after processing
- **WHEN** a quality check runs for a `high` video published 1 day ago, recorded at 360 pixels high, and the downloader reports 1080 available
- **THEN** the video is downloaded again, its file is replaced under the same name, its recorded height becomes 1080, its subtitles, chapters and watch position are unchanged, and its quality is Done (1080 reaches the `high` settlement height)

#### Scenario: Nothing better yet
- **WHEN** a quality check runs for a young video recorded at 360 pixels high and the downloader reports 360 available
- **THEN** nothing is downloaded, the check time is recorded and the quality stays Pending

#### Scenario: Video turned old
- **WHEN** a quality check runs for a video published 8 days ago, recorded at 720 pixels high, and the downloader reports 720 available
- **THEN** nothing is downloaded and the quality is Done

#### Scenario: Re-download fails
- **WHEN** a quality check finds a higher height but the re-download fails
- **THEN** the existing file and recorded fields are unchanged, one failed attempt is counted, and the check is re-queued for its next attempt without failing the task

#### Scenario: Re-download no better
- **WHEN** a quality check for a video recorded at 360 pixels high finds 1080 available but the re-download lands at 240 pixels high (YouTube served it a worse format list)
- **THEN** the existing file and recorded height are kept, the re-download is discarded, one failed attempt is counted, and the task completes without error

#### Scenario: Degraded probe
- **WHEN** a quality check for a video published 30 days ago, recorded at 360 pixels high, probes 360 available while the downloader reports the SABR-only streaming signal
- **THEN** nothing is downloaded, the quality is Retrying with one failed attempt

#### Scenario: Degraded re-download still taller
- **WHEN** a quality check for a video published 8 days ago, recorded at 360 pixels high, finds 1080 available and the re-download lands at 720 with the SABR-only streaming signal
- **THEN** the file is replaced, the recorded height becomes 720, the quality is Retrying with one failed attempt

#### Scenario: Video reset while the upgrade downloads
- **WHEN** a video is reset for redownload or deleted while its quality upgrade is downloading
- **THEN** the upgrade places no file and records nothing

## ADDED Requirements

### Requirement: Download Skipped For a Video Not Due
The system SHALL, when a download task runs for a video that is not due for a download, not download it and not change the video or its files. For a Downloaded, Failed or Unavailable video the task SHALL complete; for a Retrying video whose next attempt is not due yet the task SHALL be re-queued to run when that attempt is due.

#### Scenario: Download task for an already-downloaded video
- **WHEN** a download task runs for a video whose status is Downloaded
- **THEN** no download is attempted, the video is left unchanged, and the task completes without error

#### Scenario: Download task for an unavailable or failed video
- **WHEN** a download task runs for a video whose status is Unavailable or Failed
- **THEN** no download is attempted, the video is left unchanged, and the task completes without error

#### Scenario: Download task for a retrying video not due yet
- **WHEN** a download task runs for a Retrying video whose next attempt is due in 2 hours
- **THEN** no download is attempted, the video is left unchanged, and the task is re-queued to run in 2 hours

#### Scenario: Download task for a retrying video that is due
- **WHEN** a download task runs for a Retrying video whose next attempt is due
- **THEN** the video is downloaded

## REMOVED Requirements

### Requirement: Automatic Retry On Download Failure
**Reason**: Replaced by the `video-retries` policy: the video's own backoff (10 min, 1 h, 6 h, then daily, giving up after 11 attempts) replaces the task queue's 5-attempt sequence plus unlimited 24 h reconcile recovery.
**Migration**: Stored `ERRORED_RETRYING` videos become `RETRYING` with 1 failed attempt; stored `ERRORED` videos become `RETRYING` with 4, so their next attempt is due 24 h after they last errored; `EXCLUDED` becomes `UNAVAILABLE`.

### Requirement: Download Skipped For an Already-Settled Video
**Reason**: Replaced by "Download Skipped For a Video Not Due": a Retrying video is downloaded once its next attempt is due.
**Migration**: None.
