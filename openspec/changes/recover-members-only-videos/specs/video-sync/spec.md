## MODIFIED Requirements

### Requirement: Sync Schedules the Download
A video sync SHALL schedule a download of the video when the video is due for one: its download status is Pending or In Progress, it is Retrying and its next attempt is due (see `video-retries`), or it is Unavailable for a recheckable reason or with no recorded reason (see `video-download`). It SHALL NOT schedule one for a Downloaded or Failed video, an Unavailable video whose reason is permanent, nor for a Retrying video whose next attempt is not due yet, nor add a second download for a video that already has one pending or running. The download SHALL use its playlist's or channel's quality and output directory as they are when it runs, not when it was scheduled.

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
- **WHEN** a video sync runs for a Downloaded or Failed video, or a video Unavailable because it was removed by the uploader
- **THEN** no download is scheduled

#### Scenario: Members-only video
- **WHEN** a video sync runs for a video Unavailable as members-only, with no download pending or running
- **THEN** a download of the video is scheduled to run immediately

#### Scenario: Retrying download not due yet
- **WHEN** a video sync runs for a video whose download is Retrying with its next attempt due in 2 hours, with no download pending
- **THEN** no download is scheduled and the video is left unchanged

#### Scenario: Retrying download due with its retry lost
- **WHEN** a video sync runs for a video whose download is Retrying and due, with no download pending or running
- **THEN** a download of the video is scheduled to run immediately and the video keeps its failed attempts
