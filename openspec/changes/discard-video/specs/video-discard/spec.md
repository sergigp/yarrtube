# Spec Delta

## Purpose

Lets the user stop a video from ever being downloaded (e.g. one too big for the container to download), keep seeing it as a deliberate choice, and undo that choice.

## ADDED Requirements

### Requirement: Discard A Video
The system SHALL let a client discard a video (`POST /api/videos/{id}/discard`, answering 204) whose download is Pending, Retrying or Failed. Its download SHALL become Discarded with no failed attempts and no failure reason, and every task of the video pending in the queue SHALL be removed. Discarding a Discarded video SHALL answer 204 and change nothing.

#### Scenario: Discard a retrying download
- **WHEN** a client discards a video whose download is Retrying with 3 failed attempts and a download task pending in 6 hours
- **THEN** the request answers 204, its download is Discarded with no failed attempts and no failure, and no task of the video is pending

#### Scenario: Discard a pending video
- **WHEN** a client discards a video whose download is Pending with a thumbnail fetch and a download pending
- **THEN** its download is Discarded and neither task is pending

#### Scenario: Discard a failed download
- **WHEN** a client discards a video whose download is Failed with 11 failed attempts
- **THEN** its download is Discarded with no failed attempts

#### Scenario: Discard twice
- **WHEN** a client discards a video whose download is already Discarded
- **THEN** the request answers 204 and the video is unchanged

### Requirement: Only An Undownloaded Video Can Be Discarded
Discarding a video whose download is In Progress, Downloaded or Unavailable SHALL answer 409 and change nothing. Discarding an unknown video SHALL answer 404.

#### Scenario: Download in progress
- **WHEN** a client discards a video whose download is In Progress
- **THEN** the request answers 409 and the video and its tasks are unchanged

#### Scenario: Already downloaded
- **WHEN** a client discards a Downloaded video
- **THEN** the request answers 409 and its file is kept

#### Scenario: Unavailable
- **WHEN** a client discards a video whose download is Unavailable
- **THEN** the request answers 409

#### Scenario: Unknown video
- **WHEN** a client discards a video id that doesn't exist
- **THEN** the request answers 404

### Requirement: A Discarded Video Is Never Attempted
No step SHALL be attempted for a video whose download is Discarded: a video sync SHALL schedule nothing for it, and a download task that runs for it SHALL download nothing and complete without being re-queued. A channel or playlist reconcile that still finds the video SHALL keep its download Discarded.

#### Scenario: Video sync
- **WHEN** a video sync runs for a Discarded video with no thumbnail
- **THEN** no task of the video is scheduled

#### Scenario: Stale download task
- **WHEN** a download task runs for a Discarded video
- **THEN** nothing is downloaded, the video stays Discarded and the task completes without being re-queued

#### Scenario: Channel resync
- **WHEN** a channel reconcile finds a Discarded video still among the channel's uploads, under a new title
- **THEN** the video's title is updated and its download stays Discarded with no download scheduled

### Requirement: Undo A Discard
The system SHALL let a client undo a discard (`DELETE /api/videos/{id}/discard`, answering 204). A Discarded download SHALL become Pending with no failed attempts and the video SHALL be synced now, so its download is scheduled. Undoing for a video that isn't Discarded SHALL answer 204 and change nothing. Undoing for an unknown video SHALL answer 404.

#### Scenario: Undo
- **WHEN** a client undoes the discard of a Discarded video
- **THEN** the request answers 204, its download is Pending with no failed attempts and a sync of the video is scheduled to run now

#### Scenario: Nothing to undo
- **WHEN** a client undoes the discard of a Downloaded video
- **THEN** the request answers 204 and the video is unchanged and no task is scheduled

#### Scenario: Unknown video
- **WHEN** a client undoes the discard of a video id that doesn't exist
- **THEN** the request answers 404
