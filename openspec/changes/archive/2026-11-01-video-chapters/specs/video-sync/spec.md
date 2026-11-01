# Spec Delta

## ADDED Requirements

### Requirement: Sync Schedules the Chapters Fetch
A video sync SHALL schedule a chapters fetch for the video when it is Downloaded and its chapters were never checked, unless a chapters fetch is already pending or running for it. A chapters fetch SHALL NOT run at the same time as another download, thumbnail fetch, subtitle fetch or sync of the same video.

#### Scenario: Downloaded video with unchecked chapters
- **WHEN** a video sync runs for a Downloaded video whose chapters were never checked
- **THEN** a chapters fetch for the video is scheduled to run immediately

#### Scenario: Chapters already checked
- **WHEN** a video sync runs for a Downloaded video recorded as having chapters, or as having none
- **THEN** no chapters fetch is scheduled

#### Scenario: Video not downloaded yet
- **WHEN** a video sync runs for a Pending video whose chapters were never checked
- **THEN** no chapters fetch is scheduled

#### Scenario: Chapters fetch already queued
- **WHEN** a video sync runs for a Downloaded video with unchecked chapters while a chapters fetch for it is pending
- **THEN** no second chapters fetch is scheduled

## MODIFIED Requirements

### Requirement: Sync Never Redoes a Finished Step
Every step a video sync schedules SHALL be decided from the video's stored state at the time the sync runs, so running a video sync again for the same video SHALL only schedule the steps that are still needed. A step that failed SHALL NOT cause an earlier, finished step to be scheduled again.

#### Scenario: Rerun after the download succeeded
- **WHEN** a video sync runs for a Downloaded video with a recorded thumbnail, checked chapters and Missing subtitles
- **THEN** only a subtitle fetch is scheduled, and neither a download, a thumbnail fetch nor a chapters fetch

#### Scenario: Rerun with nothing left to do
- **WHEN** a video sync runs for a Downloaded video with a recorded thumbnail, Fetched subtitles and checked chapters
- **THEN** nothing is scheduled
