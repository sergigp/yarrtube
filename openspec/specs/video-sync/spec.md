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
During each playlist or channel reconcile pass, the system SHALL apply the video sync decision to every video of the playlist or channel stored before the pass, except a video the same pass reset for redownload or recovered from Errored, for which only its download SHALL be scheduled.

#### Scenario: Stored video missing its subtitles
- **WHEN** a reconcile pass runs for a channel with a Downloaded video whose subtitles are Missing and no subtitle fetch is pending
- **THEN** a subtitle fetch for that video is scheduled

#### Scenario: Video reset for redownload
- **WHEN** a reconcile pass resets a Downloaded video whose file is missing
- **THEN** a download of that video is scheduled and no thumbnail fetch is scheduled for it in that pass

#### Scenario: Video newly added by the same pass
- **WHEN** a reconcile pass adds a new video to a playlist
- **THEN** the pass itself schedules no step for it, and its steps come from the video sync scheduled for its addition
