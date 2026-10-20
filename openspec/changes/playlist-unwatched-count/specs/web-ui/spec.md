# Spec Delta

## ADDED Requirements

### Requirement: Unwatched Badge On Sidebar Playlists
Each playlist row in the sidebar SHALL display a badge with the playlist's unwatched video count when that count is above 0 and not hidden, and no badge otherwise. After an action in the application that changes a playlist's unwatched count or its hide setting, the badge and the playlist's sidebar position SHALL update promptly, without waiting for the periodic refresh. The same actions as for channel badges apply, plus marking the playlist watched and hiding or showing its count.

#### Scenario: Playlist with unwatched videos
- **WHEN** a playlist has 3 downloaded, unwatched videos and its count is not hidden
- **THEN** its sidebar row displays a badge showing 3

#### Scenario: Playlist with nothing unwatched
- **WHEN** a playlist has no downloaded, unwatched videos
- **THEN** its sidebar row displays no badge

#### Scenario: Hidden count shows no badge
- **WHEN** a playlist has 3 downloaded, unwatched videos and its count is hidden
- **THEN** its sidebar row displays no badge

#### Scenario: Playlist badge updates after watching
- **WHEN** a user finishes watching one of a playlist's unwatched videos, from the playlist's view or from a channel that also tracks it
- **THEN** the playlist's badge count decreases promptly without a page reload

#### Scenario: Playlist badge updates after marking watched
- **WHEN** a user marks a playlist watched from its sidebar row or its detail view
- **THEN** the playlist's badge disappears promptly and the playlist moves into the caught-up group

### Requirement: Hide Playlist Unwatched Count
Each playlist's sidebar row menu and page header "⋮" menu SHALL offer an action labeled "Hide unwatched count" when the playlist's count is shown, or "Show unwatched count" when it is hidden. The action SHALL change that setting without confirmation. When changing it fails, the application SHALL tell the user and leave the playlist as it was. Channel menus SHALL offer neither action.

#### Scenario: Hiding a playlist's count from the sidebar
- **WHEN** a user chooses "Hide unwatched count" from a playlist row's menu
- **THEN** the row's badge disappears, the playlist sorts among the caught-up playlists, and the menu now offers "Show unwatched count"

#### Scenario: Showing a playlist's count from its page header
- **WHEN** a user chooses "Show unwatched count" from the page header "⋮" menu of a playlist whose count is hidden
- **THEN** the playlist's sidebar badge shows its unwatched count, and the menu now offers "Hide unwatched count"

#### Scenario: Changing the setting fails
- **WHEN** the request to change a playlist's hide setting fails
- **THEN** the application tells the user, and the playlist's badge and menu stay as they were

#### Scenario: Channel menus have no hide count action
- **WHEN** a user opens a channel row's menu or a channel's page header menu
- **THEN** it offers neither "Hide unwatched count" nor "Show unwatched count"

### Requirement: Sidebar Row Menu
Each channel or playlist sidebar row SHALL have an always-visible menu control, not hover-dependent, offering sync, mark watched, "Edit settings" (opening that entry's edit dialog) and delete. Playlist rows SHALL also offer the home exclusion action ("Exclude from home"/"Include in home", without confirmation, refreshing home) and the hide/show unwatched count action. The unwatched badge SHALL sit at the row's trailing edge next to the menu control. Only deleting SHALL need confirmation.

#### Scenario: Badge sits next to the menu control
- **WHEN** a row displays an unwatched badge
- **THEN** the badge sits at the row's trailing edge, next to the menu control, with no reserved blank space between them

#### Scenario: Marking watched takes effect without confirmation
- **WHEN** a user chooses the mark-watched action from a channel or playlist row's menu
- **THEN** it takes effect without a confirmation prompt

#### Scenario: Changing the home setting fails
- **WHEN** the request to exclude a playlist from home or include it again fails
- **THEN** the application tells the user and leaves the playlist as it was

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

#### Scenario: Marking a playlist watched from the sidebar
- **WHEN** a user chooses the mark-watched action from a playlist row's menu
- **THEN** the application marks that playlist watched, its unwatched badge disappears, and the badges of channels sharing its videos drop accordingly

#### Scenario: Editing a channel's settings from the sidebar
- **WHEN** a user chooses "Edit settings" from a channel row's menu
- **THEN** the edit channel dialog opens for that channel, and saving it behaves as when opened from the channel's page header

#### Scenario: Editing a playlist's settings from the sidebar
- **WHEN** a user chooses "Edit settings" from a playlist row's menu
- **THEN** the edit playlist dialog opens for that playlist, and saving it behaves as when opened from the playlist's page header

#### Scenario: Channel row menu order
- **WHEN** a user opens a channel row's menu
- **THEN** it offers, in order, sync, mark-watched, "Edit settings" and delete

#### Scenario: Playlist row menu order
- **WHEN** a user opens a playlist row's menu
- **THEN** it offers, in order, sync, mark-watched, the exclude or include in home action, the hide or show unwatched count action, "Edit settings" and delete

#### Scenario: Excluding a playlist from home from the sidebar
- **WHEN** a user chooses "Exclude from home" from the menu of a playlist shown on home
- **THEN** the playlist's videos leave the home view, and the row's menu now offers "Include in home"

#### Scenario: Including a playlist in home from the sidebar
- **WHEN** a user chooses "Include in home" from the menu of a playlist excluded from home
- **THEN** the playlist's videos appear in the home view's sections they qualify for, and the row's menu now offers "Exclude from home"

#### Scenario: Channel rows have no home exclusion action
- **WHEN** a user opens a channel row's menu
- **THEN** it offers neither "Exclude from home" nor "Include in home"

## MODIFIED Requirements

### Requirement: Unwatched Badge On Sidebar Channels
Each channel row in the sidebar SHALL display a badge with the channel's unwatched video count when that count is above 0, and no badge when it is 0. The badge SHALL update without a manual page reload. After an action in the application that changes a channel's unwatched count, the badge and the channel's sidebar position SHALL update promptly, without waiting for the periodic refresh. Such actions are: a video becoming watched or unwatched during playback, marking a single video watched, marking the channel watched, marking a playlist sharing its videos watched, syncing, and adding or deleting a channel. During playback, the channel and playlist lists SHALL be refetched only when a progress report changes the video's watched state.

#### Scenario: Channel with unwatched videos
- **WHEN** a channel has 3 downloaded, unwatched videos
- **THEN** its sidebar row displays a badge showing 3

#### Scenario: Channel with nothing unwatched
- **WHEN** a channel has no downloaded, unwatched videos
- **THEN** its sidebar row displays no badge

#### Scenario: Badge updates after watching
- **WHEN** a user finishes watching one of a channel's unwatched videos
- **THEN** the channel's badge count decreases promptly without a page reload

#### Scenario: Progress that leaves the watched state unchanged
- **WHEN** a progress report during playback leaves the video's watched state as it was
- **THEN** the application refetches neither the channel list nor the playlist list because of it

#### Scenario: Badge updates after marking watched
- **WHEN** a user marks a channel watched from its sidebar row or its detail view
- **THEN** the channel's badge disappears promptly and the channel moves into the caught-up group

#### Scenario: Badge updates after marking a video watched
- **WHEN** a user marks one of a channel's unwatched videos watched from a video menu
- **THEN** the channel's badge count decreases by one promptly without a page reload

### Requirement: Detail View Page Header
A playlist or channel detail view SHALL open with a page header showing the playlist's or channel's name, with the channel's avatar beside it for channels. Below the name it SHALL show a summary of the number of videos and, when above 0, the number of unwatched videos, also for a playlist whose count is hidden. The header SHALL provide a visible sync (reconcile) control and, beside it, an always-visible vertical "⋮" control opening an actions menu. The menu SHALL offer, for both, an action to mark the entry watched and an "Edit settings" action that opens the edit channel or playlist dialog; for playlists also an action labeled "Exclude from home" when the playlist is shown on home or "Include in home" when it is excluded, which changes that setting without confirmation, and the hide or show unwatched count action; and, for both, last and set apart from the others, an action to delete the entry. Deleting SHALL require confirmation and, once confirmed, SHALL return the user to the home view. When changing the home setting fails, the application SHALL tell the user and leave the playlist as it was. On viewports narrower than the small breakpoint, the sync control SHALL show as an icon only, keeping its accessible label.

#### Scenario: Channel page header
- **WHEN** a user opens a channel's detail view
- **THEN** the page header shows the channel's avatar, its name, a video count summary, a sync control, and a "⋮" menu offering mark-watched, "Edit settings" and delete

#### Scenario: Playlist page header
- **WHEN** a user opens a playlist's detail view
- **THEN** the page header shows the playlist's name, a video count summary, a sync control, and a "⋮" menu offering mark-watched, the exclude or include in home action, the hide or show unwatched count action, "Edit settings" and delete, with no avatar

#### Scenario: Playlist summary includes its unwatched count
- **WHEN** a user opens the detail view of a playlist with 3 videos, 2 of them downloaded and unwatched
- **THEN** the summary shows "3 videos · 2 unwatched", whether or not the playlist's count is hidden

#### Scenario: Marking a playlist watched from its view
- **WHEN** a user chooses the mark-watched action from the page header's "⋮" menu in a playlist's detail view
- **THEN** every downloaded video in the list shows a tick, and the playlist's sidebar badge disappears

#### Scenario: Editing a playlist's settings from its page header
- **WHEN** a user chooses "Edit settings" from a playlist's page header "⋮" menu
- **THEN** the edit playlist dialog opens for that playlist

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

### Requirement: Sidebar Playlist Ordering
The sidebar's Playlists section SHALL list playlists with a shown unwatched count above 0 first, ordered by unwatched count from highest to lowest, with equal counts ordered by name, ignoring case. The other playlists, including those whose count is hidden, SHALL follow, ordered by name, ignoring case.

#### Scenario: Playlists listed alphabetically
- **WHEN** the tracked playlists are "watch later", "Courses" and "Ambient", none with a shown unwatched count above 0
- **THEN** the Playlists section lists them in the order "Ambient", "Courses", "watch later"

#### Scenario: Unread playlists come first, most unread first
- **WHEN** the tracked playlists are "Ambient" (0 unwatched), "Courses" (2 unwatched), "watch later" (7 unwatched) and "Zen" (0 unwatched)
- **THEN** the Playlists section lists them in the order "watch later", "Courses", "Ambient", "Zen"

#### Scenario: Equal unwatched counts are ordered by name
- **WHEN** "zeta" and "Echo" both have 3 unwatched videos
- **THEN** "Echo" is listed before "zeta"

#### Scenario: Hidden counts sort as caught up
- **WHEN** "Music" has 40 unwatched videos with its count hidden, and "Courses" has 2 unwatched videos with its count shown
- **THEN** "Courses" is listed first, and "Music" is listed among the caught-up playlists at its alphabetical position

#### Scenario: A playlist's position follows its count
- **WHEN** a user watches enough of a playlist's videos that its unwatched count drops to 0
- **THEN** the playlist moves into the caught-up group at its alphabetical position

## REMOVED Requirements

### Requirement: Sidebar Row Actions
**Reason**: Playlist rows now offer mark-watched and the hide or show unwatched count action, so the "Playlist rows have no mark-watched action" scenario no longer holds. The requirement is replaced by "Sidebar Row Menu".
**Migration**: None for users; "Sidebar Row Menu" carries every other scenario unchanged.
