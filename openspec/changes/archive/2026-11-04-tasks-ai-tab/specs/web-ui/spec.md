## MODIFIED Requirements

### Requirement: Tasks View Tabs
The tasks view SHALL group its tasks into tabs by task family, in this order: **Active**, **Downloads**, **Syncs**, **AI**, and **Other**. The **AI** tab SHALL show only while AI is enabled in the preferences, and SHALL NOT show while preferences are loading or failed to load. Each tab SHALL show, beside its name, a count of the tasks it currently contains. A task belongs to a tab as follows:

- **Active**: every task whose status is `running`, regardless of its type.
- **Downloads**: video download and thumbnail fetch tasks.
- **Syncs**: playlist reconciliation, channel reconciliation and video sync tasks.
- **AI**: video summary tasks, while the **AI** tab shows.
- **Other**: every task that belongs to none of Downloads, Syncs or a shown AI tab, including Plex collection reconciliation, the yt-dlp self-update, deletion of a video file or of a playlist's or channel's files, and any task type the view does not otherwise recognise.

Every task SHALL appear in exactly one of Downloads, Syncs, AI, or Other, and additionally in **Active** while it is running. The view SHALL open on the **Active** tab. Each tab SHALL list its tasks in the view's usual order (running first, then tasks already due, then tasks scheduled for later). Each task row SHALL show an icon that reflects the kind of work the task does (download, sync, deletion, AI, or maintenance), so rows of different kinds within **Other** remain distinguishable.

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
- **THEN** each tab's count matches the number of tasks it lists, and the counts of every tab but Active add up to the total number of tasks

#### Scenario: No Cleanup or All tab
- **WHEN** a user opens the tasks view
- **THEN** no Cleanup tab and no All tab are offered

#### Scenario: Active tab is empty
- **WHEN** no task is running
- **THEN** the Active tab shows a message that nothing is running rather than an empty area, and the other tabs still list their tasks

#### Scenario: AI tab lists video summaries
- **WHEN** AI is enabled and the tasks include video summaries
- **THEN** the AI tab shows between Syncs and Other and lists them, and the Other tab doesn't

#### Scenario: AI tab with no tasks
- **WHEN** AI is enabled and no video summary is queued
- **THEN** the AI tab shows with a count of 0

#### Scenario: AI turned off with summaries still queued
- **WHEN** AI is off and the tasks include video summaries
- **THEN** no AI tab shows and the Other tab lists them

#### Scenario: Preferences not loaded
- **WHEN** preferences are loading or failed to load
- **THEN** no AI tab shows and the Other tab lists any video summaries
