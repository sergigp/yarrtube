## MODIFIED Requirements

### Requirement: Tasks View Describes Every Task Type
The tasks view SHALL describe every task it lists in plain language and SHALL NEVER display a task's raw type identifier. Each task family SHALL be described in human terms, including playlist, channel, video, and Plex collection syncs, video downloads and thumbnail and subtitle fetches, file and container deletions, and the yt-dlp self-update. When a name the description would include could not be resolved, the description SHALL use a generic placeholder in its place rather than omitting the description.

#### Scenario: Plex collection sync is described in plain language
- **WHEN** the tasks view lists a Plex collection reconciliation task
- **THEN** it is described in plain language and not by its raw type identifier

#### Scenario: yt-dlp self-update is described in plain language
- **WHEN** the tasks view lists a yt-dlp self-update task
- **THEN** it is described in plain language and not by its raw type identifier

#### Scenario: No raw type identifier is ever shown
- **WHEN** the tasks view lists a task of any type
- **THEN** its row shows a human-readable description and never a raw snake_case type string

#### Scenario: Video sync is described in plain language
- **WHEN** the tasks view lists a video sync task whose video and playlist or channel names were resolved
- **THEN** it is described as syncing that video in that playlist or channel, and not by its raw type identifier

### Requirement: Tasks View Tabs
The tasks view SHALL group its tasks into tabs by task family, in this order: **Active**, **Downloads**, **Syncs**, and **Other**. Each tab SHALL show, beside its name, a count of the tasks it currently contains. A task belongs to a tab as follows:

- **Active**: every task whose status is `running`, regardless of its type.
- **Downloads**: video download and thumbnail fetch tasks.
- **Syncs**: playlist reconciliation, channel reconciliation and video sync tasks.
- **Other**: every task that belongs to neither Downloads nor Syncs, including Plex collection reconciliation, the yt-dlp self-update, deletion of a video file or of a playlist's or channel's files, and any task type the view does not otherwise recognise.

Every task SHALL appear in exactly one of Downloads, Syncs, or Other, and additionally in **Active** while it is running. The view SHALL open on the **Active** tab. Each tab SHALL list its tasks in the view's usual order (running first, then tasks already due, then tasks scheduled for later). Each task row SHALL show an icon that reflects the kind of work the task does (download, sync, deletion, or maintenance), so rows of different kinds within **Other** remain distinguishable.

#### Scenario: View opens on the Active tab
- **WHEN** a user opens the tasks view
- **THEN** the Active tab is selected and lists only the tasks whose status is `running`

#### Scenario: Running task appears under Active regardless of type
- **WHEN** a sync task is running
- **THEN** it appears in the Active tab as well as in the Syncs tab

#### Scenario: Downloads tab lists download and thumbnail tasks
- **WHEN** the tasks include video downloads and thumbnail fetches
- **THEN** the Downloads tab lists those tasks and no sync or other tasks

#### Scenario: Syncs tab lists only playlist and channel reconciles
- **WHEN** the tasks include playlist, channel, and Plex collection reconciliations and a video sync
- **THEN** the Syncs tab lists the playlist and channel reconciliations and the video sync, and not the Plex collection reconciliation

#### Scenario: Other tab collects everything else
- **WHEN** the tasks include a Plex collection reconciliation, a yt-dlp self-update, a video-file deletion, a deleted container's file cleanup, and a task of an unrecognised type
- **THEN** the Other tab lists all of them and no download or playlist/channel sync tasks

#### Scenario: Counts match listed tasks
- **WHEN** tasks of several families are present
- **THEN** each tab's count matches the number of tasks it lists, and the Downloads, Syncs, and Other counts add up to the total number of tasks

#### Scenario: No Cleanup or All tab
- **WHEN** a user opens the tasks view
- **THEN** no Cleanup tab and no All tab are offered

#### Scenario: Active tab is empty
- **WHEN** no task is running
- **THEN** the Active tab shows a message that nothing is running rather than an empty area, and the other tabs still list their tasks
