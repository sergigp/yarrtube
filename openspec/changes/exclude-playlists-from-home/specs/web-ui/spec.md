## MODIFIED Requirements

### Requirement: Sidebar Row Actions
Each tracked channel or playlist row in the sidebar SHALL provide an always-visible menu control that does not depend on hover. The menu SHALL offer an action to trigger a sync (reconcile) and an action to delete the entry. For channel rows it SHALL also offer an action to mark the channel watched. For playlist rows it SHALL also offer an action labeled "Exclude from home" when the playlist is shown on home, or "Include in home" when it is excluded, which changes that setting without confirmation and refreshes the home view's sections. When changing the setting fails, the application SHALL tell the user and leave the playlist as it was. The row's unwatched badge SHALL sit at the row's trailing edge, next to the menu control, with no reserved blank space between them. Deleting SHALL require the user to confirm before it takes effect. Marking a channel watched SHALL take effect without confirmation. Deleting the channel or playlist whose detail view is currently open SHALL return the user to the home view.

#### Scenario: Opening a row's menu on a touch device
- **WHEN** a user on a touch device taps a sidebar row's menu control
- **THEN** a menu opens listing the row's actions, without navigating to the row's detail view

#### Scenario: Syncing from the sidebar
- **WHEN** a user chooses the sync action from a sidebar row's menu
- **THEN** the application triggers a reconcile for that channel or playlist

#### Scenario: Deleting from the sidebar requires confirmation
- **WHEN** a user chooses the delete action from a sidebar row's menu
- **THEN** the application prompts for confirmation before deleting that channel or playlist

#### Scenario: Deleting the currently viewed entry
- **WHEN** a user confirms deletion of the channel or playlist whose detail view is currently open
- **THEN** the application navigates to the home view

#### Scenario: Marking a channel watched from the sidebar
- **WHEN** a user chooses the mark-watched action from a channel row's menu
- **THEN** the application marks that channel watched, and its unwatched badge disappears

#### Scenario: Playlist rows have no mark-watched action
- **WHEN** a user opens a playlist row's menu
- **THEN** the menu offers sync, the exclude or include in home action, and delete, and no mark-watched action

#### Scenario: Excluding a playlist from home from the sidebar
- **WHEN** a user chooses "Exclude from home" from the menu of a playlist shown on home
- **THEN** the playlist's videos leave the home view, and the row's menu now offers "Include in home"

#### Scenario: Including a playlist in home from the sidebar
- **WHEN** a user chooses "Include in home" from the menu of a playlist excluded from home
- **THEN** the playlist's videos appear in the home view's sections they qualify for, and the row's menu now offers "Exclude from home"

#### Scenario: Channel rows have no home exclusion action
- **WHEN** a user opens a channel row's menu
- **THEN** it offers neither "Exclude from home" nor "Include in home"

### Requirement: Add Dialog Download Options
Both add dialogs SHALL present video quality and the folder name inside a collapsed "Advanced options" section that is not expanded by default; the add channel dialog SHALL also present the video limit there, and the add playlist dialog SHALL also present an "Exclude from home" checkbox there, unchecked by default, whose value is sent as the playlist's `exclude_from_home` setting. The parent folder SHALL NOT be inside "Advanced options"; it is chosen through the always-visible "Save to" list. The video quality control SHALL be labeled "Video quality" and SHALL offer a tooltip explaining that it controls the download resolution and that a lower resolution reduces storage use. In the add channel dialog, the video limit field SHALL default to 3 and SHALL accept whole numbers from 1 to 1000.

#### Scenario: Advanced options start collapsed
- **WHEN** a user opens either add dialog
- **THEN** the video quality control, the folder name and, in the add channel dialog, the video limit field are hidden inside a collapsed "Advanced options" section

#### Scenario: Parent folder outside advanced options
- **WHEN** a user opens either add dialog without expanding "Advanced options"
- **THEN** the "Save to" list is visible and usable

#### Scenario: Video quality tooltip
- **WHEN** a user reveals the "Video quality" tooltip
- **THEN** it explains that the setting controls the download resolution and that choosing a lower resolution saves storage

#### Scenario: Channel video limit default
- **WHEN** a user opens the add channel dialog and does not change the video limit
- **THEN** the video limit field defaults to 3

#### Scenario: Channel video limit range
- **WHEN** a user enters a video limit below 1 or above 1000 in the add channel dialog
- **THEN** the dialog flags the field as invalid and does not submit the request

#### Scenario: Exclude from home defaults to unchecked
- **WHEN** a user adds a playlist without touching the "Exclude from home" checkbox
- **THEN** the playlist is created with `exclude_from_home` `false`

#### Scenario: Adding a playlist excluded from home
- **WHEN** a user checks "Exclude from home" in the add playlist dialog and submits
- **THEN** the playlist is created with `exclude_from_home` `true`

#### Scenario: Add channel dialog has no exclude from home option
- **WHEN** a user opens the add channel dialog and expands "Advanced options"
- **THEN** no "Exclude from home" checkbox is shown

### Requirement: Video Actions Menu
Every video card on the home view, every row of a playlist or channel detail view's video list except the selected video's row, and the title row of the video detail pane SHALL show an always-visible vertical "⋮" control to the right of the video's title. Activating it SHALL open a menu with a "Mark as watched" item, without selecting or navigating to the video. Choosing the item SHALL mark the video watched without confirmation. The item SHALL be disabled when the video is already watched or has not finished downloading. When marking fails, the application SHALL tell the user and leave the video as it was. On a home card whose source is a playlist, the menu SHALL also offer an item labeled `Exclude "<playlist name>" from home`, which excludes that playlist from home without confirmation and refreshes the home view's sections; when it fails, the application SHALL tell the user and leave the playlist as it was. Home cards whose source is a channel, list rows and the video detail pane SHALL NOT offer it. The selected video's row SHALL keep the space the control would take, so rows stay aligned when the selection changes.

#### Scenario: Marking a video watched from a home card
- **WHEN** a user chooses "Mark as watched" from the menu of a "Continue watching" card
- **THEN** the video leaves "Continue watching" and its channel's sidebar badge decreases, without a page reload

#### Scenario: Marking a video watched from a list row
- **WHEN** a user chooses "Mark as watched" from the menu of a row in a channel or playlist video list
- **THEN** the row's thumbnail shows a tick, and the selected video does not change

#### Scenario: Marking the selected video watched from its detail pane
- **WHEN** a user chooses "Mark as watched" from the menu in the video detail pane
- **THEN** the selected video is watched and its row in the list shows a tick

#### Scenario: The selected row leaves the menu to the detail pane
- **WHEN** a video is selected in a playlist or channel detail view
- **THEN** its row in the list shows no "⋮" control, while the detail pane shows one

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
- **WHEN** a user opens the menu of a row in a playlist's video list, or of the video detail pane
- **THEN** the menu offers "Mark as watched" only

### Requirement: Mark Channel Watched From Channel View
A channel detail view SHALL provide an action to mark the channel watched, which takes effect without confirmation. The action SHALL sit in the page header's "⋮" actions menu.

#### Scenario: Marking a channel watched from its view
- **WHEN** a user chooses the mark-watched action from the page header's "⋮" menu in a channel's detail view
- **THEN** every downloaded video in the list shows a tick, and the channel's sidebar badge disappears

### Requirement: Detail View Page Header
A playlist or channel detail view SHALL open with a page header showing the playlist's or channel's name, with the channel's avatar beside it for channels. Below the name it SHALL show a summary of the number of videos and, when above 0, the number of unwatched videos. The header SHALL provide a visible sync (reconcile) control and, beside it, an always-visible vertical "⋮" control opening an actions menu. The menu SHALL offer, for channels, an action to mark the channel watched; for playlists, an action labeled "Exclude from home" when the playlist is shown on home or "Include in home" when it is excluded, which changes that setting without confirmation; and, for both, last and set apart from the others, an action to delete the entry. Deleting SHALL require confirmation and, once confirmed, SHALL return the user to the home view. When changing the home setting fails, the application SHALL tell the user and leave the playlist as it was. On viewports narrower than the small breakpoint, the sync control SHALL show as an icon only, keeping its accessible label.

#### Scenario: Channel page header
- **WHEN** a user opens a channel's detail view
- **THEN** the page header shows the channel's avatar, its name, a video count summary, a sync control, and a "⋮" menu offering mark-watched and delete

#### Scenario: Playlist page header
- **WHEN** a user opens a playlist's detail view
- **THEN** the page header shows the playlist's name, a video count summary, a sync control, and a "⋮" menu offering the exclude or include in home action and delete, with no avatar and no mark-watched action

#### Scenario: Summary omits unwatched count when nothing is unwatched
- **WHEN** a detail view's channel or playlist has no unwatched videos
- **THEN** the summary shows only the video count

#### Scenario: Syncing from the page header
- **WHEN** a user activates the sync action in a detail view's page header
- **THEN** the application triggers a reconcile for that channel or playlist and shows it as in progress until it completes

#### Scenario: Deleting from the page header
- **WHEN** a user chooses the delete action from a detail view's page header "⋮" menu and confirms
- **THEN** the application deletes that channel or playlist and navigates to the home view

#### Scenario: Page header actions on a narrow viewport
- **WHEN** a user views a detail view on a viewport narrower than the small breakpoint
- **THEN** the sync control shows as an icon only, and the name remains visible beside it and the "⋮" control

#### Scenario: Excluding a playlist from home from its page header
- **WHEN** a user chooses "Exclude from home" from a playlist's page header "⋮" menu
- **THEN** the playlist's videos leave the home view, and the menu now offers "Include in home"

#### Scenario: Including a playlist in home from its page header
- **WHEN** a user chooses "Include in home" from the page header "⋮" menu of a playlist excluded from home
- **THEN** the playlist's videos appear in the home view's sections they qualify for, and the menu now offers "Exclude from home"
