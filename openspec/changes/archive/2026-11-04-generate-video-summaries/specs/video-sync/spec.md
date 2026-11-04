# Spec Delta

## ADDED Requirements

### Requirement: Sync Schedules the Summary
A video sync SHALL schedule a summary of the video when AI and summaries are on, its channel or playlist has `summaries_enabled` `true`, it is Downloaded, its subtitles are Fetched, its summary state is Missing, unless one is already pending or running for it.

#### Scenario: Video with subtitles in an opted-in source
- **WHEN** a video sync runs for a Downloaded video with Fetched subtitles and summary state Missing in a channel with summaries on, while AI and summaries are on
- **THEN** a summary of the video is scheduled to run immediately

#### Scenario: Source not opted in
- **WHEN** a video sync runs for the same video in a channel with summaries off
- **THEN** no summary is scheduled

#### Scenario: AI summaries off
- **WHEN** a video sync runs for the same video in a channel with summaries on, while AI or summaries are off
- **THEN** no summary is scheduled

#### Scenario: No subtitles yet
- **WHEN** a video sync runs for a Downloaded video whose subtitles are Missing or Unavailable, in an opted-in source
- **THEN** no summary is scheduled

#### Scenario: Summary settled
- **WHEN** a video sync runs for a video whose summary state is Generated or Unavailable
- **THEN** no summary is scheduled

#### Scenario: Source opted in with a stored library
- **WHEN** a reconcile pass runs for a channel that just turned summaries on, with 3 Downloaded videos with Fetched subtitles and summary state Missing
- **THEN** a summary of each of the 3 videos is scheduled

#### Scenario: Summary already queued
- **WHEN** a video sync runs for a video with a summary pending
- **THEN** no second summary is scheduled

## MODIFIED Requirements

### Requirement: Video Sync Task
The system SHALL sync a stored video through a video sync task that targets that one video. A video sync SHALL be scheduled to run immediately when the video is added to a playlist or channel, each time it finishes downloading and each time its subtitles are fetched. A video sync that runs for a video that no longer exists, or that no longer belongs to a playlist or channel, SHALL complete without scheduling anything.

#### Scenario: Video added to a playlist
- **WHEN** a reconcile pass adds a new video to a playlist
- **THEN** a video sync for that video is scheduled to run immediately

#### Scenario: Video added to a channel
- **WHEN** a reconcile pass adds a new video to a channel
- **THEN** a video sync for that video is scheduled to run immediately

#### Scenario: Video finished downloading
- **WHEN** a video's download completes successfully
- **THEN** a video sync for that video is scheduled to run immediately

#### Scenario: Subtitles fetched
- **WHEN** a video's subtitle fetch stores its subtitles
- **THEN** a video sync for that video is scheduled to run immediately

#### Scenario: Video deleted before its sync runs
- **WHEN** a video sync runs for a video whose record no longer exists
- **THEN** the sync completes successfully and schedules nothing
