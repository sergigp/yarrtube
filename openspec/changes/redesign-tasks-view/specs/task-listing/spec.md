# Spec Delta

## ADDED Requirements

### Requirement: Per-Video Tasks Carry The Video Identifier
A listed task that works on one video (download, thumbnail, subtitle and chapters fetch, quality check, summary, categorization and video sync) SHALL include the identifier of that video, taken from its stored payload, whether or not the video can still be found.

#### Scenario: Download task
- **WHEN** a client lists the non-completed tasks and a download for video `abc123` is pending
- **THEN** the returned task includes `video_id` `abc123`

#### Scenario: Video no longer exists
- **WHEN** a client lists the non-completed tasks and a thumbnail fetch's video record was deleted
- **THEN** the returned task still includes its `video_id`, without a video title

#### Scenario: Task not about a video
- **WHEN** a client lists the non-completed tasks and a yt-dlp self-update is pending
- **THEN** the returned task includes no `video_id`

### Requirement: Running Tasks Carry Their Start Time
A listed task whose status is `running` SHALL include `started_at`, the time its current attempt was claimed. A pending task SHALL include no `started_at`.

#### Scenario: Running task
- **WHEN** a client lists the non-completed tasks and a download was claimed at 20:03:10
- **THEN** the returned task has status `running` and `started_at` 20:03:10

#### Scenario: Pending task
- **WHEN** a client lists the non-completed tasks and a reconciliation is pending
- **THEN** the returned task has `started_at` null
