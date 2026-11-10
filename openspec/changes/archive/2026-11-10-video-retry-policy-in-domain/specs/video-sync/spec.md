## MODIFIED Requirements

### Requirement: Sync Schedules the Download
A video sync SHALL schedule a download of the video when the video is due for one: its download status is Pending or In Progress, or it is Retrying and its next attempt is due (see `video-retries`). It SHALL NOT schedule one for a Downloaded, Failed or Unavailable video, nor for a Retrying video whose next attempt is not due yet, nor add a second download for a video that already has one pending or running. The download SHALL use its playlist's or channel's quality and output directory as they are when it runs, not when it was scheduled.

#### Scenario: New video
- **WHEN** a video sync runs for a Pending video with no download pending or running
- **THEN** a download of the video is scheduled to run immediately

#### Scenario: Owner changed while the download is queued
- **WHEN** a video's download is queued and its playlist's or channel's quality or folder changes before it runs
- **THEN** the video is downloaded at the new quality into the new folder

#### Scenario: Download already queued
- **WHEN** a video sync runs for a Pending video that already has a download pending
- **THEN** no additional download is scheduled

#### Scenario: Settled video
- **WHEN** a video sync runs for a Downloaded, Failed or Unavailable video
- **THEN** no download is scheduled

#### Scenario: Retrying download not due yet
- **WHEN** a video sync runs for a video whose download is Retrying with its next attempt due in 2 hours, with no download pending
- **THEN** no download is scheduled and the video is left unchanged

#### Scenario: Retrying download due with its retry lost
- **WHEN** a video sync runs for a video whose download is Retrying and due, with no download pending or running
- **THEN** a download of the video is scheduled to run immediately and the video keeps its failed attempts

### Requirement: Sync Schedules the Thumbnail Fetch
A video sync SHALL schedule a thumbnail fetch for the video when it has no recorded thumbnail, its download status is neither In Progress, Unavailable, Retrying nor Failed, and its thumbnail is Pending, or Retrying and due (see `video-retries`), and no thumbnail fetch is already pending or running for it.

#### Scenario: New video without a thumbnail
- **WHEN** a video sync runs for a Pending video with no recorded thumbnail
- **THEN** a thumbnail fetch for the video is scheduled alongside its download

#### Scenario: Thumbnail already recorded
- **WHEN** a video sync runs for a video that has a recorded thumbnail
- **THEN** no thumbnail fetch is scheduled

#### Scenario: Video being downloaded
- **WHEN** a video sync runs for an In Progress video with no recorded thumbnail
- **THEN** no thumbnail fetch is scheduled

#### Scenario: Thumbnail failed recently
- **WHEN** a video sync runs for a Pending video whose thumbnail is Retrying, having failed once 3 minutes ago
- **THEN** no thumbnail fetch is scheduled

#### Scenario: Thumbnail gave up
- **WHEN** a video sync runs for a Pending video whose thumbnail is Failed
- **THEN** no thumbnail fetch is scheduled

### Requirement: Sync Schedules the Subtitle Fetch
A video sync SHALL schedule a subtitle fetch for the video when it is Downloaded, has an episode number, is stored in its season folder, its subtitles are Pending or Retrying and due for a check (see `late-video-data` "Re-Check Spacing"), unless a subtitle fetch is already pending or running for it.

#### Scenario: Freshly downloaded video
- **WHEN** a video sync runs for a Downloaded, numbered video in its season folder whose subtitles are Pending and were never checked
- **THEN** a subtitle fetch for the video is scheduled to run immediately

#### Scenario: Subtitles checked recently
- **WHEN** a video sync runs for a Downloaded video whose subtitles are Pending and were last checked 3 hours ago
- **THEN** no subtitle fetch is scheduled

#### Scenario: Subtitles already settled
- **WHEN** a video sync runs for a Downloaded video whose subtitle state is Fetched or Unavailable
- **THEN** no subtitle fetch is scheduled

### Requirement: Sync Schedules the Chapters Fetch
A video sync SHALL schedule a chapters fetch for the video when it is Downloaded, its chapters are Pending or Retrying and due for a check (see `late-video-data` "Re-Check Spacing"), unless a chapters fetch is already pending or running for it. A chapters fetch SHALL NOT run at the same time as another download, thumbnail fetch, subtitle fetch, quality check or sync of the same video.

#### Scenario: Downloaded video with unchecked chapters
- **WHEN** a video sync runs for a Downloaded video whose chapters were never checked
- **THEN** a chapters fetch for the video is scheduled to run immediately

#### Scenario: Young video's chapters checked a day ago
- **WHEN** a video sync runs for a Downloaded video recorded as having no chapters, Pending, last checked 25 hours ago
- **THEN** a chapters fetch for the video is scheduled

#### Scenario: Chapters already checked
- **WHEN** a video sync runs for a Downloaded video whose chapters are Done or Unavailable
- **THEN** no chapters fetch is scheduled

#### Scenario: Video not downloaded yet
- **WHEN** a video sync runs for a Pending video whose chapters were never checked
- **THEN** no chapters fetch is scheduled

#### Scenario: Chapters fetch already queued
- **WHEN** a video sync runs for a Downloaded video with unchecked chapters while a chapters fetch for it is pending
- **THEN** no second chapters fetch is scheduled

### Requirement: Sync Schedules the Quality Check
A video sync SHALL schedule a quality check (see `video-download`) for the video when it is Downloaded, its quality is Pending or Retrying and due for a check (see `late-video-data` "Re-Check Spacing"), unless a quality check is already pending or running for it. A quality check SHALL run in the download lane and SHALL NOT run at the same time as another download, thumbnail fetch, subtitle fetch, chapters fetch or sync of the same video.

#### Scenario: Young video below its tier, checked a day ago
- **WHEN** a video sync runs for a Downloaded video whose quality is Pending and was last checked 25 hours ago
- **THEN** a quality check for the video is scheduled

#### Scenario: Just downloaded
- **WHEN** a video sync runs for a video downloaded 10 minutes ago whose quality is Pending
- **THEN** no quality check is scheduled

#### Scenario: Quality settled
- **WHEN** a video sync runs for a Downloaded video whose quality is Done or Failed
- **THEN** no quality check is scheduled

### Requirement: Sync Schedules the Summary
A video sync SHALL schedule a summary of the video when AI and summaries are on, its channel or playlist has `summaries_enabled` `true`, it is Downloaded, its subtitles are Done, its summary is Pending, or Retrying and due (see `video-retries`), unless one is already pending or running for it.

#### Scenario: Video with subtitles in an opted-in source
- **WHEN** a video sync runs for a Downloaded video with Done subtitles and a Pending summary in a channel with summaries on, while AI and summaries are on
- **THEN** a summary of the video is scheduled to run immediately

#### Scenario: Source not opted in
- **WHEN** a video sync runs for the same video in a channel with summaries off
- **THEN** no summary is scheduled

#### Scenario: AI summaries off
- **WHEN** a video sync runs for the same video in a channel with summaries on, while AI or summaries are off
- **THEN** no summary is scheduled

#### Scenario: No subtitles yet
- **WHEN** a video sync runs for a Downloaded video whose subtitles are not Done, in an opted-in source
- **THEN** no summary is scheduled

#### Scenario: Summary settled
- **WHEN** a video sync runs for a video whose summary is Done or Failed
- **THEN** no summary is scheduled

#### Scenario: Source opted in with a stored library
- **WHEN** a reconcile pass runs for a channel that just turned summaries on, with 3 Downloaded videos with Done subtitles and a Pending summary
- **THEN** a summary of each of the 3 videos is scheduled

#### Scenario: Summary already queued
- **WHEN** a video sync runs for a video with a summary pending
- **THEN** no second summary is scheduled

#### Scenario: Summary failed recently
- **WHEN** a video sync runs for a video in an opted-in source whose summary is Retrying, having failed once 3 minutes ago, with no summary pending
- **THEN** no summary is scheduled

### Requirement: Sync Warns About a Stranded Video
A video sync SHALL log a warning for a stranded video: one whose download status is In Progress with no download pending or running for it. Its download is then scheduled like any other unsettled video's, without changing its download status and with no cooldown. A Pending video with no download pending or running (a new video, or one this sync just reset) is not stranded: its steps are scheduled with no warning.

#### Scenario: In Progress video with no download task
- **WHEN** a video sync runs for an In Progress video with no download pending or running
- **THEN** a stranded-video warning is logged and a download of the video is scheduled

#### Scenario: New video
- **WHEN** a video sync runs for a newly added Pending video with no download pending or running
- **THEN** a thumbnail fetch and a download of it are scheduled and no stranded-video warning is logged

#### Scenario: Errored-retrying video with no download task
- **WHEN** a video sync runs for a video whose download is Retrying and due, with no download pending or running
- **THEN** no stranded-video warning is logged and a download is scheduled (see "Sync Schedules the Download")

#### Scenario: Download still running
- **WHEN** a video sync runs for an In Progress video whose download is running
- **THEN** no warning is logged and no additional download is scheduled

## REMOVED Requirements

### Requirement: Sync Recovers a Permanently Failed Video
**Reason**: Replaced by the `video-retries` policy: a Retrying download is re-queued for its next attempt and becomes Failed after 11 attempts; only the user's retry brings a Failed video back.
**Migration**: Videos stored as `ERRORED` are migrated to `RETRYING` with 4 failed attempts, so their next attempt is due 24 hours after they last errored, as the recovery cooldown did, and they get 7 more daily attempts.
