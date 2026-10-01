## ADDED Requirements

### Requirement: Tasks View Tabbed By Family
The tasks view SHALL group its tasks into tabs by task family, in this order: **Active**, **Downloads**, **Syncs**, **Cleanup**, and **All**. Each tab SHALL show, beside its name, a count of the tasks it currently contains. A task belongs to a tab as follows:

- **Active**: every task whose status is `running`, regardless of its type.
- **Downloads**: video download and thumbnail fetch tasks.
- **Syncs**: playlist reconciliation, channel reconciliation, and Plex collection reconciliation tasks.
- **Cleanup**: tasks that delete a video file, a playlist's files, or a channel's files.
- **All**: every task, whatever its type or status.

A task that fits no download, sync, or cleanup family (such as the yt-dlp self-update) SHALL still appear in **All**, and in **Active** while it is running. The view SHALL open on the **Active** tab. Each tab SHALL list its tasks in the view's usual order (running first, then tasks already due, then tasks scheduled for later).

#### Scenario: View opens on the Active tab
- **WHEN** a user opens the tasks view
- **THEN** the Active tab is selected and lists only the tasks whose status is `running`

#### Scenario: Running task appears under Active regardless of type
- **WHEN** a sync task is running
- **THEN** it appears in the Active tab as well as in its own family tab and in All

#### Scenario: Downloads tab lists download and thumbnail tasks
- **WHEN** the tasks include video downloads and thumbnail fetches
- **THEN** the Downloads tab lists those tasks and no sync or cleanup tasks

#### Scenario: Syncs tab lists reconcile tasks
- **WHEN** the tasks include playlist, channel, and Plex collection reconciliations
- **THEN** the Syncs tab lists those tasks and no download or cleanup tasks

#### Scenario: Cleanup tab lists deletion tasks
- **WHEN** the tasks include a video-file deletion and a deleted container's file cleanup
- **THEN** the Cleanup tab lists those tasks and no download or sync tasks

#### Scenario: All tab lists every task
- **WHEN** tasks of several families, including a yt-dlp self-update, are present
- **THEN** the All tab lists every task, and each tab's count matches the number of tasks it lists

#### Scenario: Active tab is empty
- **WHEN** no task is running
- **THEN** the Active tab shows a message that nothing is running rather than an empty area, and the other tabs still list their tasks

### Requirement: Tasks View Search
The tasks view SHALL show a search field above the currently selected tab's list when that tab lists more than 15 tasks, and SHALL NOT show it otherwise. While the field contains text, the current tab SHALL list only the tasks whose plain-language description contains that text, ignoring case, keeping the tab's usual order. When no task in the current tab matches, the view SHALL say that nothing matches. Clearing the field SHALL restore the tab's full list. The search text SHALL apply only to the current tab; switching tabs SHALL clear it.

#### Scenario: Search field hidden for a short tab
- **WHEN** the selected tab lists 15 or fewer tasks
- **THEN** no search field is shown for it

#### Scenario: Search field shown for a long tab
- **WHEN** the selected tab lists more than 15 tasks
- **THEN** a search field is shown above its list

#### Scenario: Filtering by description
- **WHEN** a user types text that appears in some tasks' descriptions
- **THEN** the tab lists only the tasks whose description contains that text, ignoring case

#### Scenario: Nothing matches
- **WHEN** a user's search text matches no task in the current tab
- **THEN** the view says that nothing matches

#### Scenario: Clearing the search
- **WHEN** a user clears the search field
- **THEN** the tab returns to listing all of its tasks

### Requirement: Tasks View Describes Every Task Type
The tasks view SHALL describe every task it lists in plain language and SHALL NEVER display a task's raw type identifier. Each task family SHALL be described in human terms, including playlist, channel, and Plex collection syncs, video downloads and thumbnail fetches, file and container deletions, and the yt-dlp self-update. When a name the description would include could not be resolved, the description SHALL use a generic placeholder in its place rather than omitting the description.

#### Scenario: Plex collection sync is described in plain language
- **WHEN** the tasks view lists a Plex collection reconciliation task
- **THEN** it is described in plain language and not by its raw type identifier

#### Scenario: yt-dlp self-update is described in plain language
- **WHEN** the tasks view lists a yt-dlp self-update task
- **THEN** it is described in plain language and not by its raw type identifier

#### Scenario: No raw type identifier is ever shown
- **WHEN** the tasks view lists a task of any type
- **THEN** its row shows a human-readable description and never a raw snake_case type string

### Requirement: Tasks View Shows Last Error
When a task the tasks view lists has a recorded last error, the view SHALL show that error on the task's row, so a user can see why a task is being retried. A task with no recorded last error SHALL show none.

#### Scenario: Retrying task shows its last error
- **WHEN** the tasks view lists a task that has been retried and has a recorded last error
- **THEN** the task's row shows that error text

#### Scenario: Task without an error shows none
- **WHEN** the tasks view lists a task with no recorded last error
- **THEN** the task's row shows no error text
