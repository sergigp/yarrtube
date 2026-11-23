## MODIFIED Requirements

### Requirement: Video Status Reflects Download Progress
The system SHALL track a video's download status with the `video-retries` vocabulary: Pending, In Progress, Downloaded (Done), Retrying, Failed or Unavailable, stored as `PENDING`, `IN_PROGRESS`, `DOWNLOADED`, `RETRYING`, `FAILED` and `UNAVAILABLE`. A Retrying download failed and will be attempted again; the video SHALL record its failed attempts, when the last one failed and the last failure reason. A Failed download gave up after its last allowed attempt and is attempted again only when the user retries the video. A download becomes Unavailable when an attempt fails for an unavailability reason (see "Permanently Unavailable Videos Are Excluded"), and the video SHALL keep that reason. An Unavailable download for a permanent reason is final and is never retried, not even by the user's retry; one for a recheckable reason is attempted again on every video sync (see "Recheckable Unavailable Videos Are Attempted Again").

#### Scenario: Download starts
- **WHEN** the system begins downloading a Pending or Retrying video
- **THEN** the video's status becomes In Progress

#### Scenario: Download succeeds
- **WHEN** a video download completes successfully
- **THEN** the video's status becomes Downloaded and its failed attempts and failure reason are cleared

#### Scenario: Download fails with attempts remaining
- **WHEN** a video download attempt fails for a reason that is not an unavailability reason, and it is not the 11th consecutive failure
- **THEN** the video's status becomes Retrying, its failed attempts grow by one, and its failure time and reason are recorded

#### Scenario: Download fails permanently
- **WHEN** a video's download fails for the 11th consecutive time
- **THEN** the video's status becomes Failed

#### Scenario: Download fails for a diagnosed permanent reason
- **WHEN** a video download attempt fails and the diagnosed reason (see "Failed Downloads Are Diagnosed") is an unavailability reason — members-only content, a private video, a copyright/claimed-content block, a region block, a removed video, or a terminated account
- **THEN** the video's status becomes Unavailable with its failed attempts cleared and that reason recorded, however many attempts failed before

### Requirement: Permanently Unavailable Videos Are Excluded
The system SHALL mark a video's download Unavailable (formerly "excluded"), instead of Retrying, when the diagnosed reason for its failed download (see "Failed Downloads Are Diagnosed") is an unavailability reason. The unavailability reasons are either recheckable — members-only content (requiring a paid channel membership `yt-dlp` cannot satisfy) and a private video, both of which a creator may later make public — or permanent: a copyright or claimed-content block, a region/country block, a removed or no-longer-available video, and a video whose associated account has been terminated. An Unavailable video's download attempt SHALL be treated as a settled, non-failing outcome: the download task SHALL complete without error so the task queue neither retries nor dead-letters it. A failure whose diagnosed reason is NOT one of these — including an undetermined or merely generic reason (such as a bare "Video unavailable" with no further detail), or a network, timeout, HTTP server, rate-limit, bot-verification, or fragment/merge error — SHALL NOT make the video Unavailable, and SHALL be retried under the `video-retries` policy.

#### Scenario: A diagnosed permanent reason excludes the video
- **WHEN** a video download attempt fails and the diagnosed reason is one of the unavailability reasons (members-only, private, copyright/claimed-content block, region block, removed, or terminated account)
- **THEN** the system marks the video Unavailable, records the reason, and completes the download task without a failure, so no retry is scheduled and the task is not dead-lettered

#### Scenario: A generic "Video unavailable" with no diagnosed cause is not excluded
- **WHEN** a video download attempt fails and the diagnosed reason is undetermined or merely a generic "Video unavailable" with no specific permanent cause
- **THEN** the system marks the video Retrying and schedules its next attempt, and does not make it Unavailable

#### Scenario: A recoverable failure is not excluded
- **WHEN** a video download attempt fails for a transient reason such as a network, timeout, HTTP server, rate-limit, bot-verification, or fragment error
- **THEN** the system marks the video Retrying and schedules its next attempt, and does not make it Unavailable

### Requirement: Download Skipped For a Video Not Due
The system SHALL, when a download task runs for a video that is not due for a download, not download it and not change the video or its files. For a Downloaded or Failed video, or an Unavailable video whose reason is permanent, the task SHALL complete; for a Retrying video whose next attempt is not due yet the task SHALL be re-queued to run when that attempt is due. An Unavailable video whose reason is recheckable or unrecorded is due (see "Recheckable Unavailable Videos Are Attempted Again").

#### Scenario: Download task for an already-downloaded video
- **WHEN** a download task runs for a video whose status is Downloaded
- **THEN** no download is attempted, the video is left unchanged, and the task completes without error

#### Scenario: Download task for an unavailable or failed video
- **WHEN** a download task runs for a video whose status is Failed, or Unavailable because it was removed by the uploader
- **THEN** no download is attempted, the video is left unchanged, and the task completes without error

#### Scenario: Download task for a retrying video not due yet
- **WHEN** a download task runs for a Retrying video whose next attempt is due in 2 hours
- **THEN** no download is attempted, the video is left unchanged, and the task is re-queued to run in 2 hours

#### Scenario: Download task for a retrying video that is due
- **WHEN** a download task runs for a Retrying video whose next attempt is due
- **THEN** the video is downloaded

## ADDED Requirements

### Requirement: Recheckable Unavailable Videos Are Attempted Again
The system SHALL treat a video whose download is Unavailable for a recheckable reason (members-only or private), or with no recorded reason, as due for a download, with no backoff and no limit on attempts. An attempt that fails for an unavailability reason SHALL leave the video Unavailable with the new reason and no failed attempt counted; one that succeeds SHALL make it Downloaded.

#### Scenario: Members-only video made public
- **WHEN** a download task runs for a video Unavailable as members-only, and the video is now public
- **THEN** the video is downloaded and its status becomes Downloaded

#### Scenario: Still members-only
- **WHEN** a download task runs for a video Unavailable as members-only, and the attempt fails as members-only again
- **THEN** the video stays Unavailable with the members-only reason and no failed attempts, and the task completes without error

#### Scenario: Recheckable video now permanently unavailable
- **WHEN** a download task runs for a video Unavailable as private, and the attempt fails because the video has been removed
- **THEN** the video stays Unavailable with the removed reason, and later download tasks for it attempt nothing

#### Scenario: Unavailable video with no recorded reason
- **WHEN** a download task runs for a video Unavailable with no recorded reason
- **THEN** a download is attempted, and the reason it fails with, if any, is recorded
