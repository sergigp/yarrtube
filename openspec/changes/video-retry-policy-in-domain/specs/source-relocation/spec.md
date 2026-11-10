## MODIFIED Requirements

### Requirement: Move Requires A Settled Source
The system SHALL accept a move only when every video of the source is Downloaded, Retrying, Failed or Unavailable, and no download, thumbnail fetch, subtitle fetch or video file deletion task for any of its videos is running or pending to run now (a task waiting for a later retry, see `video-retries`, does not block: it resolves the show folder when it runs), and no downloaded video of it is still awaiting its follow-up handling (the scheduling of its subtitle fetch). The source's pending recurring reconcile SHALL NOT block a move. Otherwise the system SHALL reject the move with a conflict, a message saying the source still has work in progress, and change nothing.

#### Scenario: Video still pending download
- **WHEN** a move is requested for a source with a video that is Pending, downloading, or Retrying with its retry due now
- **THEN** the system rejects it with a conflict and changes nothing

#### Scenario: Thumbnail or subtitle fetch queued
- **WHEN** every video of a source is Downloaded but a thumbnail or subtitle fetch for one of them is pending
- **THEN** the system rejects the move with a conflict and changes nothing

#### Scenario: Download just finished
- **WHEN** every video of a source is Downloaded and no task is queued, but a just-finished download's follow-up (scheduling its subtitle fetch) has not been handled yet
- **THEN** the system rejects the move with a conflict and changes nothing

#### Scenario: Errored and excluded videos do not block
- **WHEN** every video of a source is Downloaded except one Failed and one Unavailable video, and no per-video task is pending or running
- **THEN** the move succeeds

#### Scenario: Source with no videos
- **WHEN** a move is requested for a source that has no videos yet
- **THEN** the move succeeds

#### Scenario: Retry waiting for later does not block
- **WHEN** every video of a source is Downloaded except one whose download is Retrying with its next attempt in 6 hours
- **THEN** the move succeeds, and the retried download later lands in the new folder

### Requirement: Listings Report Work In Progress
The playlist and channel listings SHALL report, for every source, whether it has work in progress, by exactly the rule a move is accepted under (see "Move Requires A Settled Source"), so a client can tell beforehand whether a move would be refused for it.

#### Scenario: Video still pending
- **WHEN** a source with a video that is Pending or In Progress is listed
- **THEN** it is listed with work in progress

#### Scenario: Per-video task queued
- **WHEN** a source whose videos are all Downloaded, with a thumbnail or subtitle fetch queued in its show folder, is listed
- **THEN** it is listed with work in progress

#### Scenario: Download just finished
- **WHEN** a source whose videos are all Downloaded, with no task queued but a just-finished download's follow-up not yet handled, is listed
- **THEN** it is listed with work in progress

#### Scenario: Settled source
- **WHEN** a source whose videos are all Downloaded, Retrying, Failed or Unavailable, with no per-video task running or due now in its show folder, is listed
- **THEN** it is listed without work in progress
