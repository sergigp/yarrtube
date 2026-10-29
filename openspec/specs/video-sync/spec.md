# video-sync Specification

## Purpose

Owns a stored video's lifecycle: a per-video sync decides, from the video's stored state alone, which of its steps (download, thumbnail fetch, subtitle fetch) are still needed, schedules only those, and never redoes a finished step.

## Requirements

### Requirement: Video Sync Task
The system SHALL sync a stored video through a video sync task that targets that one video. A video sync SHALL be scheduled to run immediately when the video is added to a playlist or channel and each time it finishes downloading. A video sync that runs for a video that no longer exists, or that no longer belongs to a playlist or channel, SHALL complete without scheduling anything.

#### Scenario: Video added to a playlist
- **WHEN** a reconcile pass adds a new video to a playlist
- **THEN** a video sync for that video is scheduled to run immediately

#### Scenario: Video added to a channel
- **WHEN** a reconcile pass adds a new video to a channel
- **THEN** a video sync for that video is scheduled to run immediately

#### Scenario: Video finished downloading
- **WHEN** a video's download completes successfully
- **THEN** a video sync for that video is scheduled to run immediately

#### Scenario: Video deleted before its sync runs
- **WHEN** a video sync runs for a video whose record no longer exists
- **THEN** the sync completes successfully and schedules nothing

### Requirement: Sync Schedules the Download
A video sync SHALL schedule a download of the video, with its playlist's or channel's quality and output directory, when the video's download status is Pending, In Progress or Errored Retrying. It SHALL NOT schedule one for a Downloaded, Excluded or Errored video, nor add a second download for a video that already has one pending or running.

#### Scenario: New video
- **WHEN** a video sync runs for a Pending video with no download pending or running
- **THEN** a download of the video is scheduled to run immediately, with its owner's quality and output directory

#### Scenario: Download already queued
- **WHEN** a video sync runs for a Pending video that already has a download pending
- **THEN** no additional download is scheduled

#### Scenario: Settled video
- **WHEN** a video sync runs for a Downloaded, Excluded or Errored video
- **THEN** no download is scheduled

### Requirement: Sync Schedules the Thumbnail Fetch
A video sync SHALL schedule a thumbnail fetch for the video when it has no recorded thumbnail, its download status is neither In Progress, Excluded nor Errored, and no thumbnail fetch is already pending or running for it.

#### Scenario: New video without a thumbnail
- **WHEN** a video sync runs for a Pending video with no recorded thumbnail
- **THEN** a thumbnail fetch for the video is scheduled alongside its download

#### Scenario: Thumbnail already recorded
- **WHEN** a video sync runs for a video that has a recorded thumbnail
- **THEN** no thumbnail fetch is scheduled

#### Scenario: Video being downloaded
- **WHEN** a video sync runs for an In Progress video with no recorded thumbnail
- **THEN** no thumbnail fetch is scheduled

### Requirement: Sync Schedules the Subtitle Fetch
A video sync SHALL schedule a subtitle fetch for the video when it is Downloaded, has an episode number, is stored in its season folder and its subtitle state is Missing, unless a subtitle fetch is already pending or running for it.

#### Scenario: Freshly downloaded video
- **WHEN** a video sync runs for a Downloaded, numbered video in its season folder whose subtitles are Missing
- **THEN** a subtitle fetch for the video is scheduled to run immediately

#### Scenario: Subtitles already settled
- **WHEN** a video sync runs for a Downloaded video whose subtitle state is Fetched or Unavailable
- **THEN** no subtitle fetch is scheduled

### Requirement: Sync Never Redoes a Finished Step
Every step a video sync schedules SHALL be decided from the video's stored state at the time the sync runs, so running a video sync again for the same video SHALL only schedule the steps that are still needed. A step that failed SHALL NOT cause an earlier, finished step to be scheduled again.

#### Scenario: Rerun after the download succeeded
- **WHEN** a video sync runs for a Downloaded video with a recorded thumbnail and Missing subtitles
- **THEN** only a subtitle fetch is scheduled, and neither a download nor a thumbnail fetch

#### Scenario: Rerun with nothing left to do
- **WHEN** a video sync runs for a Downloaded video with a recorded thumbnail and Fetched subtitles
- **THEN** nothing is scheduled

### Requirement: Reconcile Passes Sync Stored Videos
During each playlist or channel reconcile pass, the system SHALL run the video sync of every video stored for the playlist or channel once its membership is up to date, including the videos the same pass newly added. A sync that fails for one video SHALL NOT stop the pass from syncing the others.

#### Scenario: Stored video missing its subtitles
- **WHEN** a reconcile pass runs for a channel with a Downloaded video whose subtitles are Missing and no subtitle fetch is pending
- **THEN** a subtitle fetch for that video is scheduled

#### Scenario: Video reset for redownload
- **WHEN** a reconcile pass runs for a channel with a Downloaded video whose file is missing
- **THEN** the video is reset to Pending and a thumbnail fetch and a download of it are scheduled

#### Scenario: Video newly added by the same pass
- **WHEN** a reconcile pass adds a new video to a playlist
- **THEN** a thumbnail fetch and a download of it are scheduled once, even though the video sync scheduled for its addition also runs

#### Scenario: One video fails to sync
- **WHEN** a reconcile pass syncs two videos and the first sync fails
- **THEN** the second video is still synced and the pass completes

### Requirement: Sync Resets a Video Whose File Is Broken
A video sync SHALL reset a Downloaded video whose recorded file is missing from its owner's show directory or is not an mp4 file: back to Pending, keeping its episode number, clearing its recorded filename, thumbnail filename, quality, duration and sync time, and resetting its subtitle state to Missing with no subtitle filename. The sync SHALL then schedule the steps that reset video needs.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a video sync runs for a Downloaded video whose recorded file is not present in its owner's show directory
- **THEN** the video is reset to Pending with its episode number kept and its file, thumbnail, quality and subtitle fields cleared, and a thumbnail fetch and a download of it are scheduled

#### Scenario: Downloaded video's file is not an mp4
- **WHEN** a video sync runs for a Downloaded video whose recorded file is an `.mkv`
- **THEN** the video is reset to Pending and a thumbnail fetch and a download of it are scheduled

#### Scenario: Downloaded video's file is healthy
- **WHEN** a video sync runs for a Downloaded video whose recorded mp4 file is present
- **THEN** the video is not reset

### Requirement: Sync Recovers a Permanently Failed Video
A video sync SHALL reset a video that has been permanently Errored for at least 24 hours back to Pending, the same way as a broken file, and schedule the steps it then needs. A video Errored less than 24 hours earlier SHALL be left Errored. There is no limit on how many times a video may be recovered. An Excluded video SHALL never be recovered.

#### Scenario: Video errored for a day or more
- **WHEN** a video sync runs for a video that has been permanently Errored for 24 hours or more
- **THEN** the video is reset to Pending and a thumbnail fetch and a download of it are scheduled

#### Scenario: Video errored less than a day ago
- **WHEN** a video sync runs for a video that became permanently Errored less than 24 hours earlier
- **THEN** the video stays Errored and nothing is scheduled for it

#### Scenario: Video errors again after being recovered
- **WHEN** a recovered video fails and is permanently Errored again
- **THEN** the first video sync running at least 24 hours after it errored recovers it again

#### Scenario: Excluded video
- **WHEN** a video sync runs for a video Excluded however long ago
- **THEN** the video stays Excluded and nothing is scheduled for it

### Requirement: Sync Repairs Missing Metadata
A video sync SHALL regenerate the metadata of a Downloaded video with a healthy file that has no metadata recorded as populated, without re-downloading it, resolving its `sorttitle` from its playlist position or, for a channel video, its publish date. When such a video is still stored in the previous layout, the sync SHALL also move its file set into its season folder and record the new filenames. A failed regeneration SHALL leave the video unchanged and be retried by its next sync.

#### Scenario: Downloaded video has no recorded metadata
- **WHEN** a video sync runs for a Downloaded playlist video with no metadata recorded as populated
- **THEN** its metadata is generated again with its `sorttitle` from its playlist position, without re-downloading its file

#### Scenario: Downloaded video already has recorded metadata
- **WHEN** a video sync runs for a Downloaded video that already has metadata recorded as populated
- **THEN** its metadata is not regenerated

#### Scenario: Video still in the previous layout
- **WHEN** a video sync regenerates the metadata of a Downloaded video whose recorded file is still in a per-video folder
- **THEN** its file, thumbnail and NFO are moved under the episode base name in its season folder, the empty per-video folder is removed, and the new filenames are recorded

#### Scenario: Metadata still unavailable
- **WHEN** a video sync runs for a Downloaded video with no recorded metadata and YouTube returns no metadata for it
- **THEN** the video is left unchanged

### Requirement: Sync Warns About a Stranded Video
A video sync SHALL log a warning for a stranded video: one whose download status is In Progress or Errored Retrying with no download pending or running for it. Its download is then scheduled like any other unsettled video's, without changing its download status and with no cooldown. A Pending video with no download pending or running (a new video, or one this sync just reset or recovered) is not stranded: its steps are scheduled with no warning.

#### Scenario: In Progress video with no download task
- **WHEN** a video sync runs for an In Progress video with no download pending or running
- **THEN** a stranded-video warning is logged and a download of the video is scheduled

#### Scenario: New video
- **WHEN** a video sync runs for a newly added Pending video with no download pending or running
- **THEN** a thumbnail fetch and a download of it are scheduled and no stranded-video warning is logged

#### Scenario: Errored-retrying video with no download task
- **WHEN** a video sync runs for an Errored Retrying video with no download pending or running
- **THEN** a stranded-video warning is logged, a download is scheduled, and the video stays Errored Retrying

#### Scenario: Download still running
- **WHEN** a video sync runs for an In Progress video whose download is running
- **THEN** no warning is logged and no additional download is scheduled

### Requirement: Concurrent Syncs Of One Video Are Serialized
Two video syncs of the same video SHALL NOT run at the same time: a sync started while another sync of that video is running SHALL wait for it to finish, then run in full. Syncs of different videos SHALL NOT wait for each other.

#### Scenario: Pass and event sync overlap
- **WHEN** a reconcile pass syncs a video while a video sync scheduled by its download finishing starts for the same video
- **THEN** the second sync starts only once the first finished, and decides from the state the first left
