# Spec Delta

## ADDED Requirements

### Requirement: Tasks View Filters
The tasks view SHALL list every task in one list and offer filter chips above it, in this order: **All**, **Downloads**, **Syncs**, **AI**, **Plex & upkeep**, each with a count of its tasks. **All** SHALL be selected when the view opens. Selecting a chip SHALL list only its tasks. A chip other than **All** with no tasks SHALL be hidden; when the selected chip is hidden because its tasks are all gone, **All** SHALL be selected. Every task SHALL belong to exactly one chip other than **All**.

#### Scenario: View opens on All
- **WHEN** a user opens the tasks view while 72 tasks are pending and none is running
- **THEN** the **All** chip is selected and all 72 tasks are listed

#### Scenario: Filtering by a chip
- **WHEN** a user selects the **Downloads** chip
- **THEN** only download-family tasks are listed, and the chip is marked as selected and pressed for assistive technology

#### Scenario: Chip with no tasks
- **WHEN** no task belongs to the **AI** family
- **THEN** no **AI** chip is shown

#### Scenario: Selected chip drains
- **WHEN** a user has selected **Downloads** and the last download task finishes
- **THEN** the **Downloads** chip is gone, **All** is selected and every remaining task is listed

#### Scenario: Counts
- **WHEN** tasks of several families are present
- **THEN** each chip's count matches the tasks it lists, and the counts of every chip but **All** add up to the **All** count

### Requirement: Tasks View Task Families
A task SHALL belong to one family. **Downloads**: video download, thumbnail, subtitle and chapters fetch, and quality check tasks. **Syncs**: playlist reconciliation, channel reconciliation and video sync tasks. **AI**: video summary and categorization tasks, only while AI is enabled in the loaded preferences. **Plex & upkeep**: every other task, including Plex syncs, the yt-dlp self-update, file deletions, AI tasks while AI is not enabled, and unrecognised task types.

#### Scenario: Plex and maintenance tasks
- **WHEN** the tasks include a Plex watch sync, a Plex label sync, a yt-dlp self-update and a task of an unrecognised type
- **THEN** all four are listed under **Plex & upkeep**

#### Scenario: AI tasks while AI is off
- **WHEN** AI is off, or preferences are loading or failed to load, and a video summary is queued
- **THEN** the summary is listed under **Plex & upkeep** and no **AI** chip is shown

#### Scenario: AI tasks while AI is on
- **WHEN** AI is enabled and a video categorization is queued
- **THEN** it is listed under **AI**

### Requirement: Tasks View Status Line
Under its heading the tasks view SHALL show one sentence saying what is happening now: while tasks run, how many are running and how many more are scheduled; while none runs, "Nothing running." followed by the next task due and when.

#### Scenario: Idle with tasks scheduled
- **WHEN** no task is running and the earliest pending task, a Plex watch sync, is due in 2 minutes
- **THEN** the status line reads "Nothing running. Next up in 2 min: Sync watch state with Plex."

#### Scenario: A download is running
- **WHEN** one video download is running and 71 other tasks are pending
- **THEN** the status line reads "Downloading 1 video. 71 more tasks are scheduled." with a live indicator

#### Scenario: Several kinds running
- **WHEN** a download and a channel reconciliation are running and 10 other tasks are pending
- **THEN** the status line reads "Running 2 tasks. 10 more tasks are scheduled." with a live indicator

#### Scenario: Next task already due
- **WHEN** no task is running and the earliest pending task is already due
- **THEN** the status line names that task and says it is starting soon

### Requirement: Tasks View Sections
The tasks view SHALL group the listed tasks into sections, in this order, hiding an empty one: **Running now** (running tasks), **Needs attention** (pending tasks with a recorded last error), **Next hour** (other pending tasks due within 60 minutes, or already due), **Later** (the rest). Within a section tasks SHALL be ordered by run time. Rows SHALL show no status badge.

#### Scenario: Prod snapshot
- **WHEN** no task runs, a download with a last error is due in 5 minutes, 4 tasks are due within the hour and 52 later
- **THEN** the view shows **Needs attention** with the download, **Next hour** with the 4, and **Later** with the 52, and no **Running now** section

#### Scenario: Running task
- **WHEN** a download is running
- **THEN** it is listed under **Running now** and in no other section

#### Scenario: No status badges
- **WHEN** the view lists pending, due and running tasks
- **THEN** no row shows a "pending", "queued" or "running" badge

### Requirement: Tasks View Run Times
Each scheduled row SHALL show its run time as a clock time in the browser's locale, prefixed with the day when not today, followed by the time left ("20:20 · in 17 min"). A pending task already due SHALL say "Starting soon". A running row SHALL say how long ago it started.

#### Scenario: Due later today
- **WHEN** a task is due at 20:20 and it is 20:03
- **THEN** its row shows "20:20" and "in 17 min"

#### Scenario: Due tomorrow
- **WHEN** a task is due tomorrow at 09:10
- **THEN** its row shows that it is due tomorrow at 09:10

#### Scenario: Already due
- **WHEN** a pending task's run time has passed
- **THEN** its row says "Starting soon"

#### Scenario: Running task
- **WHEN** a task started running 12 seconds ago
- **THEN** its row says it started 12 s ago

### Requirement: Tasks View Sync Batches
Pending playlist and channel reconciliations without a last error SHALL be shown as one batch row when, ordered by run time, each is due within 10 minutes of the previous one, and there are at least two. A batch SHALL name how many sources it checks, its first sources with their avatars, and expand to list each source with its kind.

#### Scenario: Batch of 16
- **WHEN** 16 playlist and channel reconciliations are due between 20:20 and 20:22
- **THEN** one row reads "Check 16 channels and playlists for new videos", shows 20:20, names the first three sources and "13 more", and shows avatars

#### Scenario: Expanding a batch
- **WHEN** a user expands a batch
- **THEN** it lists every source in it with its name and "Channel" or "Playlist", and collapsing it hides them again

#### Scenario: Lone reconciliation
- **WHEN** a channel reconciliation has no other reconciliation due within 10 minutes of it
- **THEN** it is shown as its own row, "Check <channel name> for new videos"

### Requirement: Tasks View Describes Scheduled And Running Work
The tasks view SHALL describe a pending task as what it will do, in the base form ("Check 3Blue1Brown for new videos", "Download <video> from <channel>", "Sync watch state with Plex", "Update category labels in Plex", "Update yt-dlp"), and a running task as what it is doing, in the "-ing" form ("Downloading …"). It SHALL NOT show the words "pending" or "queued".

#### Scenario: Scheduled reconciliation
- **WHEN** a channel reconciliation for "3Blue1Brown" is pending
- **THEN** it is described as "Check 3Blue1Brown for new videos"

#### Scenario: Running reconciliation
- **WHEN** that reconciliation is running
- **THEN** it is described as "Checking 3Blue1Brown for new videos"

#### Scenario: Scheduled yt-dlp update
- **WHEN** a yt-dlp self-update is pending
- **THEN** it is described as "Update yt-dlp"

### Requirement: Tasks View Needs Attention
A task under **Needs attention** SHALL be shown as a card with its description, a plain-language reason derived from its last error, and when it will be tried again ("tries again at 20:09, in 5 min"). A **Show error** control SHALL reveal the full raw error text; **Hide error** SHALL hide it.

#### Scenario: YouTube refuses a download
- **WHEN** a download's last error is "ERROR: unable to download video data: HTTP Error 403: Forbidden" and it is due again at 20:09
- **THEN** its card says YouTube refused the request (403) and that Yarrtube tries again at 20:09, without showing the raw text

#### Scenario: Unrecognised error
- **WHEN** a task's last error matches no known pattern
- **THEN** the card shows the error's first line without a leading "ERROR:" prefix

#### Scenario: Showing the raw error
- **WHEN** a user activates **Show error** on a card
- **THEN** the full raw error text is shown, and the control reads **Hide error**

### Requirement: Retry From Tasks View
A **Needs attention** card for a task that works on one video SHALL offer **Retry now**, which retries the video. While it is in progress it SHALL be disabled; on success the list SHALL refresh; on failure the card SHALL say the retry failed. The card SHALL NOT offer a way to stop retrying: a failing step gives up on its own.

#### Scenario: Retry now
- **WHEN** a user activates **Retry now** on a failing download's card
- **THEN** the application retries that video and refreshes the list

#### Scenario: Action fails
- **WHEN** the retry request fails
- **THEN** the card says the retry failed and its actions can be used again

### Requirement: Run Now From Tasks View
Every pending task that does not work on one video SHALL offer **Run now**; a batch SHALL offer **Sync all now**, and each source in an expanded batch **Sync now**. Each SHALL ask the server to run those tasks now, disable itself while in progress, refresh the list on success, and say the action failed on failure. Running tasks and per-video tasks SHALL NOT offer them.

#### Scenario: Run a Plex sync now
- **WHEN** a user activates **Run now** on a pending Plex watch sync
- **THEN** the application asks to run that task now and refreshes the list

#### Scenario: Sync a batch now
- **WHEN** a user activates **Sync all now** on a batch of 16 reconciliations
- **THEN** the application asks to run all 16 tasks now

#### Scenario: Sync one source now
- **WHEN** a user expands a batch and activates **Sync now** on "chess"
- **THEN** the application asks to run only chess's reconciliation now

#### Scenario: Failing non-video task
- **WHEN** a pending Plex watch sync with a last error is shown under **Needs attention**
- **THEN** its card offers **Run now**, which asks to run that task now

#### Scenario: Per-video task
- **WHEN** the view lists a pending thumbnail fetch
- **THEN** its row offers no **Run now**

#### Scenario: Run now fails
- **WHEN** a run-now request fails
- **THEN** the row says the action failed and its action can be activated again

### Requirement: Tasks View Buttons Name Their Task
Every action button in the tasks view (**Run now**, **Sync now**, **Sync all now**, **Show sources** / **Hide sources**, **Retry now**, **Show error** / **Hide error**) SHALL be described to assistive technology by the task, batch or source it acts on, so that buttons with the same name can be told apart.

#### Scenario: Two Run now buttons
- **WHEN** the view lists a pending Plex watch sync and a pending yt-dlp update, each with **Run now**
- **THEN** a screen reader announces each **Run now** with its task's description, "Sync watch state with Plex" or "Update yt-dlp"

### Requirement: Tasks View On A Phone
On a phone-width screen the tasks view SHALL keep each chip's visible name, letting the chip row scroll sideways rather than wrap or clip, SHALL stack each row's time above its description, and every control SHALL be at least 44 pixels tall.

#### Scenario: Chips on a phone
- **WHEN** the tasks view is shown 390 pixels wide
- **THEN** every chip shows its name and count, and chips that do not fit can be scrolled into view

#### Scenario: Touch targets
- **WHEN** the tasks view is shown 390 pixels wide
- **THEN** **Retry now**, **Run now**, **Sync now** and each chip are at least 44 pixels tall

### Requirement: Tasks View With No Tasks
When no task is pending or running, the tasks view SHALL say "All quiet. Nothing is scheduled." instead of chips and sections.

#### Scenario: Empty queue
- **WHEN** the task listing is empty
- **THEN** the view shows the "Tasks" heading and "All quiet. Nothing is scheduled.", and no chips

## MODIFIED Requirements

### Requirement: Tasks View Search
The tasks view SHALL always show a search field beside the filter chips. While the field contains text, the view SHALL list only the tasks of the selected chip whose plain-language description or source name contains that text, ignoring case, keeping the usual sections and order; a batch SHALL list only its matching sources. When nothing matches, the view SHALL say that nothing matches. Clearing the field SHALL restore the full list. Changing the chip SHALL keep the search text.

#### Scenario: Search field hidden for a short tab
- **WHEN** the selected chip lists 3 tasks
- **THEN** a search field is still shown

#### Scenario: Search field shown for a long tab
- **WHEN** the selected chip lists 68 tasks
- **THEN** a search field is shown

#### Scenario: Filtering by description
- **WHEN** a user types text that appears in some tasks' descriptions
- **THEN** only the tasks whose description contains that text, ignoring case, are listed

#### Scenario: Filtering a batch
- **WHEN** a user types "jobava" while a batch of 16 reconciliations includes three sources whose names contain "Jobava"
- **THEN** only those three sources are listed

#### Scenario: Nothing matches
- **WHEN** a user's search text matches no task in the selected chip
- **THEN** the view says that nothing matches

#### Scenario: Clearing the search
- **WHEN** a user clears the search field
- **THEN** the view lists all of the selected chip's tasks again

### Requirement: Tasks View Describes Subtitle Fetches
The tasks view SHALL describe a subtitle fetch task in plain language, naming the video whose subtitles are being fetched and the playlist or channel it belongs to, and SHALL list it with the download tasks, the same way it describes and lists a thumbnail fetch task.

#### Scenario: Subtitle fetch task with resolvable context
- **WHEN** the tasks view lists a subtitle fetch task whose video title and playlist or channel name were resolved
- **THEN** the task is described as fetching the subtitles of that video in that playlist or channel, under the **Downloads** chip

#### Scenario: Subtitle fetch task whose video can no longer be found
- **WHEN** the tasks view lists a subtitle fetch task whose video title could not be resolved
- **THEN** the task is still described as a subtitle fetch, using a generic placeholder in place of the missing names

### Requirement: Tasks View Describes Chapter Fetches
The tasks view SHALL describe a chapters fetch task in plain language, naming the video whose chapters are being fetched and the playlist or channel it belongs to, and SHALL list it with the download tasks, the same way it describes and lists a subtitle fetch task.

#### Scenario: Chapters fetch task with resolvable context
- **WHEN** the tasks view lists a chapters fetch task whose video title and playlist or channel name were resolved
- **THEN** the task is described as fetching the chapters of that video in that playlist or channel, under the **Downloads** chip

#### Scenario: Chapters fetch task whose video can no longer be found
- **WHEN** the tasks view lists a chapters fetch task whose video title could not be resolved
- **THEN** the task is still described as a chapters fetch, using a generic placeholder in place of the missing names

### Requirement: Tasks View Describes Quality Checks
The tasks view SHALL describe a quality check task in plain language, naming the video whose quality is being checked and the playlist or channel it belongs to, and SHALL list it with the download tasks, the same way it describes and lists a chapters fetch task.

#### Scenario: Quality check task with resolvable context
- **WHEN** the tasks view lists a quality check task whose video title and playlist or channel name were resolved
- **THEN** the task is described as checking the quality of that video in that playlist or channel, under the **Downloads** chip

#### Scenario: Quality check task whose video can no longer be found
- **WHEN** the tasks view lists a quality check task whose video title could not be resolved
- **THEN** the task is still described as a quality check, using a generic placeholder in place of the missing names

### Requirement: Tasks View Title
The tasks view SHALL open with a page heading "Tasks", in the same heading style and alignment as the other views' page headers.

#### Scenario: Opening the tasks view
- **WHEN** a user opens the tasks view
- **THEN** a "Tasks" heading is shown above the status line and the filter chips, aligned with the left edge of the content area

## REMOVED Requirements

### Requirement: Tasks View Tabs
**Reason**: Tabs by task family are replaced by one sectioned list with filter chips.
**Migration**: See "Tasks View Filters", "Tasks View Task Families" and "Tasks View Sections".

### Requirement: Tasks View Tab Bar Affordance
**Reason**: There is no tab bar any more.
**Migration**: Chips follow "Tasks View Filters" and "Tasks View On A Phone"; the live indicator moves to the status line.

### Requirement: Run Sync Now From Tasks View
**Reason**: Replaced by a run-now action that brings the scheduled task itself forward, offered on every non-video task and on sync batches.
**Migration**: See "Run Now From Tasks View". The sidebar's on-demand sync is unchanged.

### Requirement: Tasks View Shows Last Error
**Reason**: A task's error now moves it to the Needs attention section, where it is explained in plain words with the raw text one click away.
**Migration**: See "Tasks View Needs Attention".
