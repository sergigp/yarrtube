## MODIFIED Requirements

### Requirement: Single Execution Per Attempt
The system SHALL poll for eligible tasks in the background and dispatch each one to exactly one handler based on its type. A task SHALL be dispatched at most once per attempt, even when several tasks run at the same time. Every task type SHALL belong to exactly one lane (see "Task Lanes"). Within a single lane, when several tasks of that lane are eligible, the system SHALL start them in ascending order of the time they became eligible to run, breaking ties by the order in which they were scheduled, except for a task that is held back by "Per-Video Mutual Exclusion" or "Exclusive yt-dlp Update". Such a task SHALL be passed over without blocking later tasks of the same lane. Tasks in different lanes SHALL NOT be ordered relative to each other.

#### Scenario: Eligible task is dispatched
- **WHEN** a task's run time has passed and it has not yet succeeded
- **THEN** the system dispatches it to the handler registered for its type

#### Scenario: Earlier-due task runs first regardless of scheduling order
- **WHEN** two tasks of the same lane are eligible and the one scheduled later has an earlier run time
- **THEN** the system starts the task with the earlier run time first

#### Scenario: Tasks with the same run time
- **WHEN** two tasks of the same lane are eligible with identical run times
- **THEN** the system starts the one that was scheduled first before the other

#### Scenario: A running task is not dispatched again
- **WHEN** a task is already running and the system polls for eligible tasks again
- **THEN** that task is not dispatched a second time while its current attempt is running

#### Scenario: Held-back task does not block its lane
- **WHEN** the earliest eligible task of a lane cannot start because another task for the same video is running
- **THEN** the system starts the next eligible task of that lane that is not held back, and starts the held-back task once the conflicting task finishes

## ADDED Requirements

### Requirement: Task Lanes
The system SHALL execute tasks in lanes, each with its own maximum number of tasks running at the same time:
- a **download lane**, containing video downloads, whose concurrency is configurable,
- a **thumbnail lane**, containing thumbnail fetches, with a concurrency of 1,
- a **light lane**, containing every other task type except thumbnail fetches and the yt-dlp update (reconciles and file deletions), with a concurrency of 1.

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

### Requirement: Configurable Download Concurrency
The system SHALL read the download lane's concurrency from the `YARRTUBE_DOWNLOAD_CONCURRENCY` environment variable at daemon startup, and SHALL default to 2 when it is unset or is not a positive integer.

#### Scenario: Download concurrency not configured
- **WHEN** the daemon starts without `YARRTUBE_DOWNLOAD_CONCURRENCY` set
- **THEN** at most 2 video downloads run at the same time

#### Scenario: Download concurrency configured
- **WHEN** the daemon starts with `YARRTUBE_DOWNLOAD_CONCURRENCY` set to a positive integer N
- **THEN** at most N video downloads run at the same time

#### Scenario: Invalid download concurrency
- **WHEN** the daemon starts with `YARRTUBE_DOWNLOAD_CONCURRENCY` set to zero, a negative number, or a non-numeric value
- **THEN** the system uses the default of 2

### Requirement: Exclusive yt-dlp Update
The system SHALL run a yt-dlp update task only while no other task is running, and SHALL NOT start any other task while that update task is running. While an eligible yt-dlp update task is waiting for running tasks to finish, the system SHALL NOT start any new task in any lane.

#### Scenario: Update becomes eligible while downloads are running
- **WHEN** a yt-dlp update task becomes eligible while one or more other tasks are running
- **THEN** the system starts no new task, waits for the running tasks to finish, and then runs the update

#### Scenario: Tasks become eligible while the update runs
- **WHEN** a yt-dlp update task is running and other tasks become eligible
- **THEN** none of them start until the update task finishes

### Requirement: Per-Video Mutual Exclusion
The system SHALL NOT run two tasks that operate on the same video at the same time. A video download and a thumbnail fetch of the same video count as operating on the same video, as do two downloads of the same video.

#### Scenario: Thumbnail fetch eligible while the same video downloads
- **WHEN** a thumbnail fetch task for a video is eligible while that video's download task is running
- **THEN** the thumbnail fetch does not start until the download task finishes

#### Scenario: Download eligible while the same video's thumbnail is fetched
- **WHEN** a download task for a video is eligible while a thumbnail fetch task for that same video is running
- **THEN** the download does not start until the thumbnail fetch finishes

#### Scenario: Tasks for different videos
- **WHEN** a download task for one video and a thumbnail fetch task for a different video are both eligible
- **THEN** both may run at the same time

### Requirement: No Duplicate Per-Video Tasks
The system SHALL NOT add a video download task, or a thumbnail fetch task, for a video that already has a task of that same type pending or running. Such a scheduling request SHALL succeed without adding a task.

#### Scenario: Download already queued for the video
- **WHEN** a download task is scheduled for a video that already has a pending or running download task
- **THEN** no additional download task is added, and the request does not fail

#### Scenario: Thumbnail fetch already queued for the video
- **WHEN** a thumbnail fetch task is scheduled for a video that already has a pending or running thumbnail fetch task
- **THEN** no additional thumbnail fetch task is added, and the request does not fail

#### Scenario: Previous task for the video already finished
- **WHEN** a download or thumbnail fetch task is scheduled for a video whose previous task of that type has already completed or been dead-lettered
- **THEN** a new task is added
