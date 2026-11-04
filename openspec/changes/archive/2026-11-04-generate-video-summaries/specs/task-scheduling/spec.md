# Spec Delta

## MODIFIED Requirements

### Requirement: Task Lanes
The system SHALL execute tasks in lanes, each with its own maximum number of tasks running at the same time:
- a **download lane**, containing video downloads, whose concurrency is configurable,
- a **thumbnail lane**, containing thumbnail fetches, with a concurrency of 1,
- a **subtitle lane**, containing subtitle fetches, with a concurrency of 1,
- a **video sync lane**, containing video syncs (see `video-sync`), with a concurrency of 1,
- a **summary lane**, containing video summaries, with a concurrency of 1,
- a **light lane**, containing every other task type except thumbnail fetches, subtitle fetches, video syncs, video summaries and the yt-dlp update (reconciles and file deletions), with a concurrency of 1.

A lane SHALL start a new eligible task whenever it has fewer tasks running than its concurrency. A long-running task in one lane SHALL NOT delay the start of eligible tasks in another lane.

#### Scenario: Downloads run in parallel up to the lane's concurrency
- **WHEN** the download lane's concurrency is 2 and three video downloads are eligible
- **THEN** two of them run at the same time, and the third starts as soon as one of the first two finishes

#### Scenario: Light tasks are not blocked by downloads
- **WHEN** the download lane is fully busy with long-running downloads and a reconcile or file-deletion task becomes eligible
- **THEN** that task starts without waiting for any download to finish

#### Scenario: Light lane runs one task at a time
- **WHEN** two light-lane tasks are eligible at the same time
- **THEN** the system runs one, and starts the other only after the first finishes

#### Scenario: Reconciles and deletes are not blocked by thumbnail fetches
- **WHEN** many thumbnail fetches are queued, one of them is running, and a reconcile or file-deletion task becomes eligible
- **THEN** that task starts without waiting for any of the thumbnail fetches to finish

#### Scenario: Thumbnail lane runs one fetch at a time
- **WHEN** two thumbnail fetches for different videos are eligible at the same time
- **THEN** the system runs one, and starts the other only after the first finishes

#### Scenario: Subtitle backfill does not block other work
- **WHEN** many subtitle fetches are queued, one of them is running, and a video download, thumbnail fetch, reconcile or file-deletion task becomes eligible
- **THEN** that task starts without waiting for any of the subtitle fetches to finish

#### Scenario: Subtitle lane runs one fetch at a time
- **WHEN** two subtitle fetches for different videos are eligible at the same time
- **THEN** the system runs one, and starts the other only after the first finishes

#### Scenario: Video syncs are not blocked by reconciles
- **WHEN** a playlist or channel reconcile is running and a video sync becomes eligible
- **THEN** the video sync starts without waiting for the reconcile to finish

#### Scenario: Video sync lane runs one sync at a time
- **WHEN** two video syncs for different videos are eligible at the same time
- **THEN** the system runs one, and starts the other only after the first finishes

#### Scenario: Summary backlog does not block other work
- **WHEN** many video summaries are queued, one of them is running, no exclusive task is eligible, and a video download, thumbnail fetch, reconcile or file-deletion task becomes eligible
- **THEN** that task starts without waiting for any of the summaries to finish

#### Scenario: Summary lane runs one summary at a time
- **WHEN** two video summaries for different videos are eligible at the same time
- **THEN** the system runs one, and starts the other only after the first finishes

### Requirement: Per-Video Mutual Exclusion
The system SHALL NOT run two tasks that operate on the same video at the same time. A video download, a thumbnail fetch, a subtitle fetch, a video summary and a video sync of the same video all count as operating on the same video, as do two tasks of the same type for the same video.

#### Scenario: Thumbnail fetch eligible while the same video downloads
- **WHEN** a thumbnail fetch task for a video is eligible while that video's download task is running
- **THEN** the thumbnail fetch does not start until the download task finishes

#### Scenario: Download eligible while the same video's thumbnail is fetched
- **WHEN** a download task for a video is eligible while a thumbnail fetch task for that same video is running
- **THEN** the download does not start until the thumbnail fetch finishes

#### Scenario: Tasks for different videos
- **WHEN** a download task for one video and a thumbnail fetch task for a different video are both eligible
- **THEN** both may run at the same time

#### Scenario: Video sync eligible while the same video downloads
- **WHEN** a video sync for a video is eligible while that video's download task is running
- **THEN** the video sync does not start until the download task finishes

#### Scenario: Video sync eligible while the same video is summarized
- **WHEN** a video sync for a video is eligible while that video's summary is running
- **THEN** the sync does not start until the summary finishes
