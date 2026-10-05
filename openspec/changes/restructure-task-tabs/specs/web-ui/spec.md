## REMOVED Requirements

### Requirement: Tasks View Tabbed By Family
**Reason**: Replaced by "Tasks View Tabs": the Cleanup and All tabs are dropped, Plex collection reconciliation leaves Syncs, and an Other catch-all tab is added.
**Migration**: Deletion tasks, Plex collection reconciliation, the yt-dlp self-update and unrecognised task types are listed under Other.

## ADDED Requirements

### Requirement: Tasks View Tabs
The tasks view SHALL group its tasks into tabs by task family, in this order: **Active**, **Downloads**, **Syncs**, and **Other**. Each tab SHALL show, beside its name, a count of the tasks it currently contains. A task belongs to a tab as follows:

- **Active**: every task whose status is `running`, regardless of its type.
- **Downloads**: video download and thumbnail fetch tasks.
- **Syncs**: playlist reconciliation and channel reconciliation tasks.
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
- **WHEN** the tasks include playlist, channel, and Plex collection reconciliations
- **THEN** the Syncs tab lists the playlist and channel reconciliations and not the Plex collection reconciliation

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

### Requirement: Tasks View Tab Bar Affordance
The tasks view's tab bar SHALL make each tab look clickable and SHALL make the selected tab unmistakable. The tab bar SHALL be drawn as a row of tabs over a horizontal rule, with the selected tab marked by an underline in the application's primary colour and its label in the full foreground colour and a heavier weight; unselected tabs SHALL show a muted label. Hovering an unselected tab SHALL visibly change its background and label colour. Each tab SHALL show an icon for its family before its name, and its count as a pill after its name, with the selected tab's pill in the primary colour. While at least one task is running, the **Active** tab SHALL show a live indicator beside its count. Each tab SHALL be at least 40 pixels tall, sized to its content rather than stretched across the full width, and SHALL show a visible focus ring when focused from the keyboard. When the tabs do not fit the available width, the tab bar SHALL scroll horizontally rather than wrap or shrink its tabs. The tab bar SHALL meet these requirements in both light and dark themes.

#### Scenario: Selected tab is marked
- **WHEN** a user views the tasks view with the Syncs tab selected
- **THEN** the Syncs tab shows the primary-coloured underline, a full-colour heavier label, and a primary-coloured count pill, and no other tab does

#### Scenario: Hovering an unselected tab
- **WHEN** a user hovers an unselected tab with a pointer
- **THEN** that tab's background and label colour change

#### Scenario: Live indicator on Active
- **WHEN** at least one task is running
- **THEN** the Active tab shows a live indicator beside its count

#### Scenario: No live indicator when idle
- **WHEN** no task is running
- **THEN** the Active tab shows no live indicator

#### Scenario: Keyboard focus
- **WHEN** a user moves keyboard focus onto a tab
- **THEN** that tab shows a visible focus ring

#### Scenario: Narrow screen
- **WHEN** the tasks view is shown on a screen too narrow for all tabs
- **THEN** the tab bar scrolls horizontally and each tab keeps its full label, icon, and count

### Requirement: Run Sync Now From Tasks View
Each playlist or channel reconciliation task listed in the **Syncs** tab SHALL offer a **Run now** action. Activating it SHALL trigger the on-demand reconcile of that task's playlist or channel — the same action the sidebar's sync offers — and SHALL leave the listed task itself and its scheduled run time unchanged. While the triggered reconcile is in progress, that row's action SHALL show that it is working and SHALL NOT accept another activation. When the reconcile completes, the tasks view SHALL refresh its list. When the reconcile fails, the view SHALL show the failure on that row without removing it. A reconciliation task whose status is `running` SHALL NOT offer Run now, since its sync is already underway. Tasks in other tabs SHALL NOT offer this action.

#### Scenario: Running a playlist sync now
- **WHEN** a user activates Run now on a playlist reconciliation task in the Syncs tab
- **THEN** the application triggers a reconcile of that playlist, and the task remains listed with its scheduled run time unchanged

#### Scenario: Running a channel sync now
- **WHEN** a user activates Run now on a channel reconciliation task in the Syncs tab
- **THEN** the application triggers a reconcile of that channel

#### Scenario: Run now while in progress
- **WHEN** a user has activated Run now on a row and the reconcile has not completed yet
- **THEN** that row's action indicates it is working and cannot be activated again

#### Scenario: Run now completes
- **WHEN** a triggered reconcile completes successfully
- **THEN** the tasks view refreshes its list of tasks

#### Scenario: Run now fails
- **WHEN** a triggered reconcile fails
- **THEN** the view shows that the sync failed, and the row's action can be activated again

#### Scenario: No Run now on a running sync
- **WHEN** the Syncs tab lists a reconciliation task whose status is `running`
- **THEN** that row offers no Run now action

#### Scenario: No Run now outside Syncs
- **WHEN** a user views the Downloads, Other, or Active tab
- **THEN** no task row offers a Run now action
