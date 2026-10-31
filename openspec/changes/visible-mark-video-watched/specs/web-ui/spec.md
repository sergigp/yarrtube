## ADDED Requirements

### Requirement: Video Detail Watched Control
The video detail pane SHALL show, in its own row below the meta line and at every viewport size, a labelled watched control for the selected video. While the video is downloaded and unwatched, it SHALL be a "Mark watched" button that marks the video watched without confirmation. Once watched, it SHALL be a muted, non-interactive "Watched" status that hints that rewatching marks it unwatched. Nothing SHALL be shown while the video has not finished downloading.

#### Scenario: Marking the selected video watched
- **WHEN** a user activates "Mark watched" in the detail pane of a downloaded, unwatched video
- **THEN** the selected video is watched, its row in the list shows a tick, and the control reads "Watched"

#### Scenario: Watched video
- **WHEN** a user selects a watched video
- **THEN** the detail pane shows "Watched" as a status that cannot be activated, with a hint that rewatching past 10% marks it unwatched

#### Scenario: Rewatching resets the control
- **WHEN** a user replays a watched video past 10% of its duration
- **THEN** the video becomes unwatched and the control reads "Mark watched" again, without a page reload

#### Scenario: Video not downloaded yet
- **WHEN** the selected video is pending, downloading or errored
- **THEN** the detail pane shows no watched control

#### Scenario: Marking fails
- **WHEN** a user activates "Mark watched" and the request fails
- **THEN** the application tells the user and the video stays unwatched

#### Scenario: Visible on a mobile viewport
- **WHEN** a user selects a downloaded, unwatched video below the desktop breakpoint
- **THEN** "Mark watched" is visible below the meta line without opening any menu, and the title keeps its full width

## MODIFIED Requirements

### Requirement: Video Actions Menu
Every video card on the home view and every row of a playlist or channel detail view's video list except the selected video's row SHALL show an always-visible vertical "⋮" control to the right of the video's title. Activating it SHALL open a menu with a "Mark as watched" item, without selecting or navigating to the video. Choosing the item SHALL mark the video watched without confirmation. The item SHALL be disabled when the video is already watched or has not finished downloading. When marking fails, the application SHALL tell the user and leave the video as it was. On a home card whose source is a playlist, the menu SHALL also offer an item labeled `Exclude "<playlist name>" from home`, which excludes that playlist from home without confirmation and refreshes the home view's sections; when it fails, the application SHALL tell the user and leave the playlist as it was. Home cards whose source is a channel and list rows SHALL NOT offer it. The video detail pane SHALL NOT show a "⋮" control; the selected video is marked watched from its watched control, as described in "Video Detail Watched Control". The selected video's row SHALL keep the space the control would take, so rows stay aligned when the selection changes.

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
- **THEN** neither its row in the list nor the detail pane shows a "⋮" control, and the detail pane shows the watched control instead

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

### Requirement: Playback Speed Control
The title row of a playlist or channel detail view's video detail pane SHALL show a playback speed control to the right of the title, labelled with the current playback speed (e.g. "1x", "1.5x"). Activating it SHALL open a menu offering the speeds 1x, 1.1x, 1.25x, 1.5x and 2x, with the current speed marked. Choosing a speed SHALL change the playing video's speed immediately without interrupting playback. The speed SHALL apply only to the selected video: selecting another video, or reloading the application, SHALL start playback at 1x. When the speed is changed through the browser's own player controls, the control SHALL show that speed, even when it is not one of the offered speeds. The control SHALL be disabled while the selected video has not finished downloading.

#### Scenario: Choosing a faster speed
- **WHEN** a user opens the speed control of a downloaded video and chooses "1.5x"
- **THEN** the video plays at 1.5 times normal speed and the control reads "1.5x"

#### Scenario: Default speed
- **WHEN** a user selects a downloaded video
- **THEN** the speed control reads "1x" and the video plays at normal speed

#### Scenario: Speed resets when another video is selected
- **WHEN** a user plays a video at 2x and then selects another video
- **THEN** the newly selected video plays at 1x and the control reads "1x"

#### Scenario: Speed resets on reload
- **WHEN** a user plays a video at 1.25x and reloads the page
- **THEN** the video plays at 1x and the control reads "1x"

#### Scenario: Speed changed from the browser's own controls
- **WHEN** a user changes the speed to 1.75x through the browser's native player menu
- **THEN** the speed control reads "1.75x" and none of the offered speeds is marked as current

#### Scenario: Video not downloaded yet
- **WHEN** the selected video is pending, downloading or errored
- **THEN** the speed control is shown disabled

### Requirement: Detail View Page Header
A playlist or channel detail view SHALL open with a page header showing the playlist's or channel's name, with the channel's avatar beside it for channels. Below the name it SHALL show a summary of the number of videos and, when above 0, the number of unwatched videos, also for a playlist whose count is hidden. The header SHALL offer, in this order, a sync (reconcile) action; for both, an action to mark the entry watched; for playlists an action labeled "Exclude from home" when the playlist is shown on home or "Include in home" when it is excluded, which changes that setting without confirmation, and the hide or show unwatched count action; for both, an "Edit settings" action that opens the edit channel or playlist dialog; and last, an action to delete the entry. At or above the desktop breakpoint every action but mark-watched and delete SHALL be a square icon-only button whose accessible name and tooltip are the action's label, followed by a vertical "⋮" control, named "More actions for <name>", whose menu offers the mark-watched action, labeled "Mark all watched", and, set apart, delete, styled as destructive. Below the desktop breakpoint the header SHALL show only that "⋮" control, always visible, whose menu offers every action, sync included, with delete set apart from the others. Deleting SHALL require confirmation and, once confirmed, SHALL return the user to the home view. When syncing or changing the home setting fails, the application SHALL tell the user and leave the entry as it was.

#### Scenario: Channel page header
- **WHEN** a user opens a channel's detail view
- **THEN** the page header shows the channel's avatar, its name, a video count summary, and the sync, mark-watched, "Edit settings" and delete actions

#### Scenario: Playlist page header
- **WHEN** a user opens a playlist's detail view
- **THEN** the page header shows the playlist's name, a video count summary, and the sync, mark-watched, exclude or include in home, hide or show unwatched count, "Edit settings" and delete actions, with no avatar

#### Scenario: Header actions on a desktop viewport
- **WHEN** a user views a playlist's detail view at or above the desktop breakpoint
- **THEN** each action but mark-watched and delete shows as a square icon-only button, hovering one shows its label in a tooltip, and a "⋮" control follows them whose menu offers "Mark all watched" and, set apart, delete

#### Scenario: Mark all watched is not a header button on desktop
- **WHEN** a user views a channel's, playlist's or category's view at or above the desktop breakpoint
- **THEN** no icon button in the header marks the entry watched; "Mark all watched" is reached only through the "⋮" menu

#### Scenario: Page header actions on a narrow viewport
- **WHEN** a user views a playlist's detail view below the desktop breakpoint
- **THEN** the header shows only the "⋮" control beside the name, and its menu offers sync, mark-watched, the home and unwatched count actions, "Edit settings" and, set apart, delete

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

### Requirement: Video Detail Title Prominence
In the video detail pane, the video's title SHALL occupy its own row, taking all of its width except what the playback speed control (to its right) and, in a channel detail view, the channel avatar (to its left) need. No other elements SHALL share that row at any viewport size.

#### Scenario: Long title on a mobile viewport
- **WHEN** a user on a mobile-width viewport selects a video with a long title
- **THEN** the title wraps across the width of the detail pane left by the speed control, and the meta line appears below it
