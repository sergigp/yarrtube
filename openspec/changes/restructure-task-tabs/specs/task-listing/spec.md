## MODIFIED Requirements

### Requirement: List Non-Completed Tasks
The system SHALL provide an HTTP endpoint that returns every scheduled task whose status is `pending` or `running`, regardless of when it is next due to run, including each task's type, status, retry count, scheduled run time, and — resolved server-side from the task's stored payload at read time — whatever context that payload makes resolvable: the name of the playlist or channel a task concerns, the title of a video being downloaded or having its thumbnail fetched, and the raw filename or output path a file-cleanup task targets. A playlist or channel reconciliation task SHALL also include the identifier of the playlist or channel it reconciles, taken from its stored payload, whether or not that playlist or channel can still be found.

#### Scenario: Non-completed tasks exist
- **WHEN** a client requests the list of non-completed tasks and one or more tasks are `pending` or `running`
- **THEN** the daemon responds with HTTP status 200 and a list containing each of those tasks

#### Scenario: No non-completed tasks exist
- **WHEN** a client requests the list of non-completed tasks and none are currently `pending` or `running`
- **THEN** the daemon responds with HTTP status 200 and an empty list

#### Scenario: A pending task is not yet due
- **WHEN** a client requests the list of non-completed tasks and a `pending` task's scheduled run time is in the future
- **THEN** that task is still included in the response

#### Scenario: Several tasks are running at the same time
- **WHEN** a client requests the list of non-completed tasks while more than one task is `running`
- **THEN** every running task is included in the response with status `running`

#### Scenario: Task concerns a tracked playlist
- **WHEN** a client requests the list of non-completed tasks and a task's payload references a playlist that is still tracked
- **THEN** the returned task includes that playlist's current name

#### Scenario: Task concerns a tracked channel
- **WHEN** a client requests the list of non-completed tasks and a task's payload references a channel that is still tracked
- **THEN** the returned task includes that channel's current name

#### Scenario: Playlist reconciliation task carries the playlist identifier
- **WHEN** a client requests the list of non-completed tasks and a playlist reconciliation task is pending or running
- **THEN** the returned task includes the identifier of the playlist it reconciles

#### Scenario: Channel reconciliation task carries the channel identifier
- **WHEN** a client requests the list of non-completed tasks and a channel reconciliation task is pending or running
- **THEN** the returned task includes the identifier of the channel it reconciles

#### Scenario: Task concerns a specific video
- **WHEN** a client requests the list of non-completed tasks and a task downloads a specific video
- **THEN** the returned task includes that video's title, and the current name of whichever playlist or channel the video belongs to

#### Scenario: Task fetches a specific video's thumbnail
- **WHEN** a client requests the list of non-completed tasks and a task fetches a specific video's thumbnail
- **THEN** the returned task includes that video's title, and the current name of whichever playlist or channel the video belongs to

#### Scenario: Task targets a specific file
- **WHEN** a client requests the list of non-completed tasks and a task deletes one video's downloaded file
- **THEN** the returned task includes the filename recorded on the task, with no playlist, channel, or video name resolved

#### Scenario: Task deletes a removed container's files
- **WHEN** a client requests the list of non-completed tasks and a task deletes every file under a deleted playlist's or deleted channel's output path
- **THEN** the returned task includes that output path, with no playlist or channel name resolved

#### Scenario: Referenced playlist or video no longer exists
- **WHEN** a client requests the list of non-completed tasks and a task's payload references a playlist, channel, or video that can no longer be found
- **THEN** the daemon still responds with HTTP status 200 and includes the task, omitting the name(s) that could not be resolved but keeping a reconciliation task's playlist or channel identifier
