# Spec Delta

## MODIFIED Requirements

### Requirement: Detail View Page Header
A playlist or channel detail view SHALL open with a page header showing the playlist's or channel's name, with the channel's avatar beside it for channels. Below the name it SHALL show a summary of the number of videos and, when above 0, the number of unwatched videos, also for a playlist whose count is hidden. The header SHALL offer, in this order, a sync (reconcile) action; for both, an action to mark the entry watched; for playlists an action labeled "Exclude from home" when the playlist is shown on home or "Include in home" when it is excluded, which changes that setting without confirmation, and the hide or show unwatched count action; for both, an "Edit settings" action that opens the edit channel or playlist dialog; for channels a link labeled "Open channel on YouTube" and for playlists a link labeled "Open playlist on YouTube", which opens the channel's or playlist's page on youtube.com in a new tab; and last, an action to delete the entry. At or above the desktop breakpoint every action but mark-watched and delete SHALL be a square icon-only button whose accessible name and tooltip are the action's label, followed by a vertical "⋮" control, named "More actions for <name>", whose menu offers the mark-watched action, labeled "Mark all watched", and, set apart, delete, styled as destructive. Below the desktop breakpoint the header SHALL show only that "⋮" control, always visible, whose menu offers every action, sync included, with delete set apart from the others. Deleting SHALL require confirmation and, once confirmed, SHALL return the user to the home view. When syncing or changing the home setting fails, the application SHALL tell the user and leave the entry as it was.

#### Scenario: Channel page header
- **WHEN** a user opens a channel's detail view
- **THEN** the page header shows the channel's avatar, its name, a video count summary, and the sync, mark-watched, "Edit settings", "Open channel on YouTube" and delete actions

#### Scenario: Playlist page header
- **WHEN** a user opens a playlist's detail view
- **THEN** the page header shows the playlist's name, a video count summary, and the sync, mark-watched, exclude or include in home, hide or show unwatched count, "Edit settings", "Open playlist on YouTube" and delete actions, with no avatar

#### Scenario: Opening the source on YouTube
- **WHEN** a user activates "Open channel on YouTube" in the page header of the channel `@somechannel`
- **THEN** a new tab opens to `https://www.youtube.com/@somechannel`

#### Scenario: Header actions on a desktop viewport
- **WHEN** a user views a playlist's detail view at or above the desktop breakpoint
- **THEN** each action but mark-watched and delete shows as a square icon-only button, hovering one shows its label in a tooltip, and a "⋮" control follows them whose menu offers "Mark all watched" and, set apart, delete

#### Scenario: Mark all watched is not a header button on desktop
- **WHEN** a user views a channel's, playlist's or category's view at or above the desktop breakpoint
- **THEN** no icon button in the header marks the entry watched; "Mark all watched" is reached only through the "⋮" menu

#### Scenario: Page header actions on a narrow viewport
- **WHEN** a user views a playlist's detail view below the desktop breakpoint
- **THEN** the header shows only the "⋮" control beside the name, and its menu offers sync, mark-watched, the home and unwatched count actions, "Edit settings", "Open playlist on YouTube" and, set apart, delete

#### Scenario: Playlist summary includes its unwatched count
- **WHEN** a user opens the detail view of a playlist with 3 videos, 2 of them downloaded and unwatched
- **THEN** the summary shows "3 videos · 2 unwatched", whether or not the playlist's count is hidden

#### Scenario: Marking a playlist watched from its view
- **WHEN** a user chooses the mark-watched action from a playlist's page header
- **THEN** every downloaded video in the list shows a tick, and the playlist's sidebar badge disappears

#### Scenario: Editing a playlist's settings from its page header
- **WHEN** a user chooses "Edit settings" from a playlist's page header
- **THEN** the edit playlist dialog opens for that playlist

#### Scenario: Summary omits unwatched count when nothing is unwatched
- **WHEN** a detail view's channel or playlist has no unwatched videos
- **THEN** the summary shows only the video count

#### Scenario: Syncing from the page header
- **WHEN** a user activates the sync action in a detail view's page header
- **THEN** the application triggers a reconcile for that channel or playlist and shows it as in progress until it completes

#### Scenario: Deleting from the page header
- **WHEN** a user chooses the delete action from a detail view's "More actions" menu and confirms
- **THEN** the application deletes that channel or playlist and navigates to the home view

#### Scenario: Excluding a playlist from home from its page header
- **WHEN** a user chooses "Exclude from home" from a playlist's page header
- **THEN** the playlist's videos leave the home view, and the header now offers "Include in home"

#### Scenario: Including a playlist in home from its page header
- **WHEN** a user chooses "Include in home" from the page header of a playlist excluded from home
- **THEN** the playlist's videos appear in the home view's sections they qualify for, and the header now offers "Exclude from home"

### Requirement: Video Detail Links To YouTube
A selected video's detail pane SHALL show, at the end of its title row and whether the details section is expanded or collapsed, an icon-only link whose accessible name and tooltip are "Open video on YouTube", which opens that video on YouTube in a new tab.

#### Scenario: Opening a video on YouTube
- **WHEN** a user activates the "Open video on YouTube" link for a selected video
- **THEN** a new tab opens to that video's page on youtube.com

#### Scenario: Visible while details are collapsed
- **WHEN** a user on a mobile-width viewport selects a video with the details section collapsed
- **THEN** the "Open video on YouTube" link is visible in the title row

### Requirement: Video Actions Menu
Every video card on the home view and every row of a playlist or channel detail view's video list, the selected video's row included, SHALL show an always-visible vertical "⋮" control to the right of the video's title. Activating it SHALL open a menu with a "Mark as watched" item, without selecting or navigating to the video. Choosing the item SHALL mark the video watched without confirmation. The item SHALL be disabled when the video is already watched or has not finished downloading. When marking fails, the application SHALL tell the user and leave the video as it was. On a home card whose source is a playlist, the menu SHALL also offer an item labeled `Exclude "<playlist name>" from home`, which excludes that playlist from home without confirmation and refreshes the home view's sections; when it fails, the application SHALL tell the user and leave the playlist as it was. Home cards whose source is a channel and list rows SHALL NOT offer it. The video detail pane SHALL NOT show a "⋮" control; the selected video can also be marked watched from its "Mark watched" button, as described in "Video Detail Watched Control".

#### Scenario: Marking a video watched from a home card
- **WHEN** a user chooses "Mark as watched" from the menu of a "Continue watching" card
- **THEN** the video leaves "Continue watching" and its channel's sidebar badge decreases, without a page reload

#### Scenario: Marking a video watched from a list row
- **WHEN** a user chooses "Mark as watched" from the menu of a row in a channel or playlist video list
- **THEN** the row's thumbnail shows a tick, and the selected video does not change

#### Scenario: Marking the selected video watched from its detail pane
- **WHEN** a user activates "Mark watched" in the video detail pane
- **THEN** the selected video is watched and its row in the list shows a tick

#### Scenario: The selected row leaves the menu to the detail pane
- **WHEN** a video is selected in a playlist or channel detail view
- **THEN** its row in the list shows a "⋮" control like every other row, and the detail pane shows none

#### Scenario: Opening the menu doesn't navigate
- **WHEN** a user opens the menu on a home card or a list row
- **THEN** the application neither navigates nor changes the selected video

#### Scenario: Already watched video
- **WHEN** a user opens the menu of a watched video
- **THEN** "Mark as watched" is shown disabled

#### Scenario: Video not downloaded yet
- **WHEN** a user opens the menu of a video that is pending, downloading or errored
- **THEN** "Mark as watched" is shown disabled

#### Scenario: Excluding a card's playlist from home
- **WHEN** a user chooses `Exclude "Bluey" from home` from the menu of a home card whose source is the playlist "Bluey"
- **THEN** every card sourced from "Bluey" leaves the home view, without a page reload

#### Scenario: Channel cards have no exclude item
- **WHEN** a user opens the menu of a home card whose source is a channel
- **THEN** the menu offers "Mark as watched" only

#### Scenario: List rows and the detail pane have no exclude item
- **WHEN** a user opens the menu of a row in a playlist's video list
- **THEN** the menu offers "Mark as watched" only, and the video detail pane offers no menu at all

### Requirement: Video Detail Watched Control
The video detail pane's title row SHALL show, to the right of the title and before the playback speed control, a "Mark watched" button for the selected video while it is downloaded and unwatched, which marks the video watched without confirmation. At or above the desktop breakpoint the button SHALL show a tick and the label "Mark watched"; below it, only the tick, keeping "Mark watched" as its accessible name. The button SHALL NOT be shown once the video is watched or while it has not finished downloading; a watched video is shown as such on the meta line, as described in "Video Detail Meta Line".

#### Scenario: Marking the selected video watched
- **WHEN** a user activates "Mark watched" in the detail pane of a downloaded, unwatched video
- **THEN** the selected video is watched, its row in the list shows a tick, the button disappears and the meta line starts with "Watched"

#### Scenario: Watched video
- **WHEN** a user selects a watched video
- **THEN** the detail pane shows no "Mark watched" button and its meta line starts with "Watched"

#### Scenario: Rewatching resets the control
- **WHEN** a user replays a watched video past 10% of its duration
- **THEN** the video becomes unwatched and the "Mark watched" button shows again, without a page reload

#### Scenario: Video not downloaded yet
- **WHEN** the selected video is pending, downloading or errored
- **THEN** the detail pane shows no "Mark watched" button

#### Scenario: Marking fails
- **WHEN** a user activates "Mark watched" and the request fails
- **THEN** the application tells the user and the video stays unwatched

#### Scenario: Visible on a mobile viewport
- **WHEN** a user selects a downloaded, unwatched video below the desktop breakpoint
- **THEN** the title row shows the button as a tick without its label, named "Mark watched" for assistive technology, and it is reachable without opening any menu

### Requirement: Video Detail Title Prominence
In the video detail pane, the video's title SHALL occupy its own row, taking all of its width except what the channel avatar (to its left, in a channel detail view) and the video's actions (to its right: the "Mark watched" button when shown, the playback speed control and the "Open video on YouTube" link) need. No other elements SHALL share that row at any viewport size.

#### Scenario: Long title on a mobile viewport
- **WHEN** a user on a mobile-width viewport selects a video with a long title
- **THEN** the title wraps across the width of the detail pane left by the video's actions, and the meta line appears below it

### Requirement: Collapsible Video Details
The video detail pane SHALL always show the video's title and its meta line. The rest of the pane (the status indicator when the video is not downloaded, the description and the storage location) SHALL sit in a section the user can expand and collapse. The storage location SHALL be the folder holding the video's file (its source's storage path joined with the file's folder, for example `channels/some-channel/Season 2026`), never the file name itself, so episode codes do not appear in the UI. This section SHALL start collapsed on viewports narrower than the desktop breakpoint and expanded at or above it. The pane SHALL NOT show a quality indicator, and SHALL NOT show a status indicator for a downloaded video.

#### Scenario: Details start collapsed on mobile
- **WHEN** a user on a mobile-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the video's title, its meta line and an expand control, and hides the description and storage location

#### Scenario: Expanding details
- **WHEN** a user activates the expand control in a collapsed detail pane
- **THEN** the description and storage location become visible, and the control collapses them again when activated

#### Scenario: Details start expanded on desktop
- **WHEN** a user on a desktop-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the title, meta line, description and storage location without needing to be expanded

#### Scenario: Storage location of a downloaded episode
- **WHEN** a downloaded video's recorded filename is `Season 2026/S2026E031501 - Title.mp4` in a channel stored at `channels/some-channel`
- **THEN** the detail pane shows `channels/some-channel/Season 2026` and not the file name

#### Scenario: Storage location of a video not yet downloaded
- **WHEN** a video has no recorded filename
- **THEN** the detail pane shows its source's storage path

#### Scenario: Downloaded video shows no status indicator
- **WHEN** a user expands the details of a downloaded video
- **THEN** no status or quality indicator is shown

#### Scenario: Pending video shows its status
- **WHEN** a user expands the details of a video that is pending, downloading, retrying or errored
- **THEN** a status indicator shows that state, and no quality indicator is shown

### Requirement: Video Detail Meta Line
The video detail pane SHALL show a meta line directly below the title, visible whether the details section is expanded or collapsed. The line SHALL read `Watched · <channel name> · Published <date> · Synced <relative time>`:
- "Watched" SHALL appear only when the video is watched.
- The channel name SHALL appear only in a playlist detail view.
- The publish time SHALL be shown as a date, without a time of day.
- The sync time SHALL be shown relative to now (for example "2d ago"), and its full date and time SHALL be available on hover. It SHALL be shown only at or above the desktop breakpoint.
- Any part whose value is absent SHALL be omitted along with its separator. When every part is absent, the line SHALL not be shown.

#### Scenario: Downloaded video in a playlist view
- **WHEN** a user on a desktop-width viewport selects a downloaded, unwatched video with generated metadata in a playlist detail view
- **THEN** the meta line shows the video's channel name, "Published" followed by its publish date, and "Synced" followed by how long ago it was synced

#### Scenario: Downloaded video in a channel view
- **WHEN** a user on a desktop-width viewport selects a downloaded, unwatched video with generated metadata in a channel detail view
- **THEN** the meta line shows "Published" with its publish date and "Synced" with how long ago it was synced, without the channel name

#### Scenario: Watched video
- **WHEN** a user selects a watched video
- **THEN** the meta line starts with "Watched", followed by the other parts

#### Scenario: Sync time hidden on a mobile viewport
- **WHEN** a user on a mobile-width viewport selects a synced video
- **THEN** the meta line shows no "Synced" part and no separator for it

#### Scenario: Meta line on a collapsed mobile pane
- **WHEN** a user on a mobile-width viewport opens a detail view with the details section collapsed
- **THEN** the meta line is visible below the title

#### Scenario: Video without metadata or sync time
- **WHEN** a user selects a pending video that has neither generated metadata nor a sync time
- **THEN** no meta line is shown
