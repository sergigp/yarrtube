## MODIFIED Requirements

### Requirement: Sync Schedules the Subtitle Fetch
A video sync SHALL schedule a subtitle fetch for the video when it is Downloaded, has an episode number, is stored in its season folder, its subtitle state is Missing and its subtitles are due for a check (never checked, or last checked at least 24 hours earlier; see `late-video-data`), unless a subtitle fetch is already pending or running for it.

#### Scenario: Freshly downloaded video
- **WHEN** a video sync runs for a Downloaded, numbered video in its season folder whose subtitles are Missing and were never checked
- **THEN** a subtitle fetch for the video is scheduled to run immediately

#### Scenario: Subtitles checked recently
- **WHEN** a video sync runs for a Downloaded video whose subtitles are Missing and were last checked 3 hours ago
- **THEN** no subtitle fetch is scheduled

#### Scenario: Subtitles already settled
- **WHEN** a video sync runs for a Downloaded video whose subtitle state is Fetched or Unavailable
- **THEN** no subtitle fetch is scheduled

### Requirement: Sync Schedules the Chapters Fetch
A video sync SHALL schedule a chapters fetch for the video when it is Downloaded, its chapters are unsettled and they are due for a check (never checked, or last checked at least 24 hours earlier; see `late-video-data`), unless a chapters fetch is already pending or running for it. A chapters fetch SHALL NOT run at the same time as another download, thumbnail fetch, subtitle fetch, quality check or sync of the same video.

#### Scenario: Downloaded video with unchecked chapters
- **WHEN** a video sync runs for a Downloaded video whose chapters were never checked
- **THEN** a chapters fetch for the video is scheduled to run immediately

#### Scenario: Young video's chapters checked a day ago
- **WHEN** a video sync runs for a Downloaded video recorded as having no chapters, unsettled, last checked 25 hours ago
- **THEN** a chapters fetch for the video is scheduled

#### Scenario: Chapters already checked
- **WHEN** a video sync runs for a Downloaded video whose chapters are settled, with or without chapters
- **THEN** no chapters fetch is scheduled

#### Scenario: Video not downloaded yet
- **WHEN** a video sync runs for a Pending video whose chapters were never checked
- **THEN** no chapters fetch is scheduled

#### Scenario: Chapters fetch already queued
- **WHEN** a video sync runs for a Downloaded video with unchecked chapters while a chapters fetch for it is pending
- **THEN** no second chapters fetch is scheduled

### Requirement: Sync Never Redoes a Finished Step
Every step a video sync schedules SHALL be decided from the video's stored state and the current time at the time the sync runs, so running a video sync again for the same video SHALL only schedule the steps that are still needed and due. A step that failed SHALL NOT cause an earlier, finished step to be scheduled again.

#### Scenario: Rerun after the download succeeded
- **WHEN** a video sync runs for a Downloaded video with a recorded thumbnail, settled chapters and quality, and Missing subtitles never checked
- **THEN** only a subtitle fetch is scheduled, and neither a download, a thumbnail fetch, a chapters fetch nor a quality check

#### Scenario: Rerun with nothing left to do
- **WHEN** a video sync runs for a Downloaded video with a recorded thumbnail, Fetched subtitles, and settled chapters and quality
- **THEN** nothing is scheduled

### Requirement: Reconcile Passes Sync Stored Videos
During each playlist or channel reconcile pass, the system SHALL run the video sync of every video stored for the playlist or channel once its membership is up to date, including the videos the same pass newly added. A sync that fails for one video SHALL NOT stop the pass from syncing the others.

#### Scenario: Stored video missing its subtitles
- **WHEN** a reconcile pass runs for a channel with a Downloaded video whose subtitles are Missing, last checked more than 24 hours ago, and no subtitle fetch is pending
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
A video sync SHALL reset a Downloaded video whose recorded file is missing from its owner's show directory or is not an mp4 file: back to Pending, keeping its episode number, clearing its recorded filename, thumbnail filename, quality, height, duration and sync time, resetting its subtitle state to Missing with no subtitle filename, and making its subtitles, chapters and quality unsettled and never checked, with no failed attempts. The sync SHALL then schedule the steps that reset video needs.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a video sync runs for a Downloaded video whose recorded file is not present in its owner's show directory
- **THEN** the video is reset to Pending with its episode number kept and its file, thumbnail, quality, height and subtitle fields cleared, and a thumbnail fetch and a download of it are scheduled

#### Scenario: Downloaded video's file is not an mp4
- **WHEN** a video sync runs for a Downloaded video whose recorded file is an `.mkv`
- **THEN** the video is reset to Pending and a thumbnail fetch and a download of it are scheduled

#### Scenario: Downloaded video's file is healthy
- **WHEN** a video sync runs for a Downloaded video whose recorded mp4 file is present
- **THEN** the video is not reset

## ADDED Requirements

### Requirement: Sync Schedules the Quality Check
A video sync SHALL schedule a quality check (see `video-download`) for the video when it is Downloaded, its quality is unsettled and due for a check (last checked at least 24 hours earlier; see `late-video-data`), unless a quality check is already pending or running for it. A quality check SHALL run in the download lane and SHALL NOT run at the same time as another download, thumbnail fetch, subtitle fetch, chapters fetch or sync of the same video.

#### Scenario: Young video below its tier, checked a day ago
- **WHEN** a video sync runs for a Downloaded video whose quality is unsettled and was last checked 25 hours ago
- **THEN** a quality check for the video is scheduled

#### Scenario: Just downloaded
- **WHEN** a video sync runs for a video downloaded 10 minutes ago whose quality is unsettled
- **THEN** no quality check is scheduled

#### Scenario: Quality settled
- **WHEN** a video sync runs for a Downloaded video whose quality is settled
- **THEN** no quality check is scheduled
