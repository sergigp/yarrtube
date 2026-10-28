## ADDED Requirements

### Requirement: Confirm Bulk Mark Watched
Marking a channel, playlist or category watched, from its page header or its sidebar row menu, SHALL first ask the user to confirm when it has 2 or more unwatched videos, counting them also when its count is hidden. The confirmation SHALL state how many videos will be marked and that this cannot be undone in bulk, with a destructive-styled confirm action. Cancelling SHALL mark nothing. With 0 or 1 unwatched videos, it SHALL take effect without confirmation.

#### Scenario: Confirming marks the entry watched
- **WHEN** a user chooses "Mark all watched" for the channel "Foo", which has 23 unwatched videos
- **THEN** a confirmation asks to mark 23 videos in "Foo" watched and says this cannot be undone in bulk
- **AND** confirming marks the channel watched and closes the confirmation

#### Scenario: Cancelling marks nothing
- **WHEN** a user chooses "Mark all watched" for a playlist with 5 unwatched videos and cancels the confirmation
- **THEN** no video is marked watched and the playlist's badge still shows 5

#### Scenario: Marking a category asks for confirmation
- **WHEN** a user chooses "Mark all watched" from the header of a category with 3 unwatched videos
- **THEN** a confirmation asks to mark 3 videos in the category watched before anything is marked

#### Scenario: One unwatched video needs no confirmation
- **WHEN** a user chooses "Mark all watched" for a channel with exactly 1 unwatched video
- **THEN** the channel is marked watched without a confirmation prompt

#### Scenario: Nothing unwatched needs no confirmation
- **WHEN** a user chooses "Mark all watched" for a playlist with no unwatched videos
- **THEN** it takes effect without a confirmation prompt

#### Scenario: Hidden count still confirms
- **WHEN** a user chooses "Mark all watched" from the menu of a playlist whose unwatched count is hidden and which has 4 unwatched videos
- **THEN** a confirmation asks to mark 4 videos watched

#### Scenario: Marking fails
- **WHEN** a user confirms marking an entry watched and the request fails
- **THEN** the confirmation stays open, shows the error, and no video is marked watched

## MODIFIED Requirements

### Requirement: Mark Channel Watched From Channel View
A channel detail view SHALL provide an action to mark the channel watched, which asks for confirmation as described in "Confirm Bulk Mark Watched". The action SHALL sit among the page header's actions.

#### Scenario: Marking a channel watched from its view
- **WHEN** a user chooses the mark-watched action from a channel's page header and confirms
- **THEN** every downloaded video in the list shows a tick, and the channel's sidebar badge disappears

### Requirement: Sidebar Row Menu
Each channel or playlist sidebar row SHALL have an always-visible menu control, not hover-dependent, offering sync, mark watched, "Edit settings" (opening that entry's edit dialog) and delete. Playlist rows SHALL also offer the home exclusion action ("Exclude from home"/"Include in home", without confirmation, refreshing home) and the hide/show unwatched count action. The unwatched badge SHALL sit at the row's trailing edge next to the menu control. Deleting SHALL need confirmation, and mark watched SHALL ask for it as described in "Confirm Bulk Mark Watched"; no other action SHALL.

#### Scenario: Badge sits next to the menu control
- **WHEN** a row displays an unwatched badge
- **THEN** the badge sits at the row's trailing edge, next to the menu control, with no reserved blank space between them

#### Scenario: Marking watched takes effect without confirmation
- **WHEN** a user chooses the mark-watched action from the menu of a channel or playlist row with 1 or no unwatched videos
- **THEN** it takes effect without a confirmation prompt

#### Scenario: Marking watched asks for confirmation
- **WHEN** a user chooses the mark-watched action from the menu of a channel or playlist row with 2 or more unwatched videos
- **THEN** the application asks for confirmation before marking it watched

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
- **WHEN** a user chooses the mark-watched action from a channel row's menu and confirms
- **THEN** the application marks that channel watched, and its unwatched badge disappears

#### Scenario: Marking a playlist watched from the sidebar
- **WHEN** a user chooses the mark-watched action from a playlist row's menu and confirms
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

### Requirement: Category View
Each category SHALL have a view at an address of its own with a header and its videos as a grid of the same cards as the home view, each YouTube video once, newest sync first. The header SHALL show the category's name, a summary of its number of sources, its number of videos and, when above 0, its number of unwatched videos, and the same actions layout as the channel and playlist views' page header, offering "Manage sources", which opens the manage sources dialog, "Mark all watched" (only when the category has sources), which marks every downloaded video of the category watched after the confirmation described in "Confirm Bulk Mark Watched", and "Delete", which deletes the category after the same confirmation as the sidebar row's "Delete" and then navigates to the home view. A category with no sources SHALL say so and offer a control that opens the manage sources dialog. A category with sources but without downloaded videos SHALL say it has no videos yet. When the category does not exist, the view SHALL say it was not found.

#### Scenario: Category videos
- **WHEN** a user opens the "Chess" view, which holds 2 sources with 14 downloaded videos, 3 of them unwatched
- **THEN** the view shows "Chess", the summary "2 sources · 14 videos · 3 unwatched", and the videos as cards, newest sync first

#### Scenario: Mark all watched
- **WHEN** a user chooses "Mark all watched" from a category view's header and confirms
- **THEN** every card shows as watched, and the category's sidebar badge disappears promptly

#### Scenario: Deleting from the category view
- **WHEN** a user chooses "Delete" from the "Chess" view's header and confirms
- **THEN** the category is deleted and the application navigates to the home view

#### Scenario: Empty category
- **WHEN** a user opens a category with no sources
- **THEN** the view says it holds no channels or playlists yet and offers a control that opens the manage sources dialog

#### Scenario: Unknown category
- **WHEN** a user opens the view of a category that does not exist
- **THEN** the view says the category was not found
