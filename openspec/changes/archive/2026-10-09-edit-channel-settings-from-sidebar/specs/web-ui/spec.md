## MODIFIED Requirements

### Requirement: Sidebar Row Actions
Each tracked channel or playlist row in the sidebar SHALL provide an always-visible menu control that does not depend on hover. The menu SHALL offer an action to trigger a sync (reconcile) and an action to delete the entry. For channel rows it SHALL also offer an action to mark the channel watched and an "Edit settings" action that opens the edit channel dialog for that channel. For playlist rows it SHALL also offer an action labeled "Exclude from home" when the playlist is shown on home, or "Include in home" when it is excluded, which changes that setting without confirmation and refreshes the home view's sections. When changing the setting fails, the application SHALL tell the user and leave the playlist as it was. The row's unwatched badge SHALL sit at the row's trailing edge, next to the menu control, with no reserved blank space between them. Deleting SHALL require the user to confirm before it takes effect. Marking a channel watched SHALL take effect without confirmation. Deleting the channel or playlist whose detail view is currently open SHALL return the user to the home view.

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

#### Scenario: Editing a channel's settings from the sidebar
- **WHEN** a user chooses "Edit settings" from a channel row's menu
- **THEN** the edit channel dialog opens for that channel, and saving it behaves as when opened from the channel's page header

#### Scenario: Channel row menu order
- **WHEN** a user opens a channel row's menu
- **THEN** it offers, in order, sync, mark-watched, "Edit settings" and delete

#### Scenario: Playlist rows have no mark-watched action
- **WHEN** a user opens a playlist row's menu
- **THEN** the menu offers sync, the exclude or include in home action, and delete, and no mark-watched action and no "Edit settings" action

#### Scenario: Excluding a playlist from home from the sidebar
- **WHEN** a user chooses "Exclude from home" from the menu of a playlist shown on home
- **THEN** the playlist's videos leave the home view, and the row's menu now offers "Include in home"

#### Scenario: Including a playlist in home from the sidebar
- **WHEN** a user chooses "Include in home" from the menu of a playlist excluded from home
- **THEN** the playlist's videos appear in the home view's sections they qualify for, and the row's menu now offers "Exclude from home"

#### Scenario: Channel rows have no home exclusion action
- **WHEN** a user opens a channel row's menu
- **THEN** it offers neither "Exclude from home" nor "Include in home"

### Requirement: Edit Channel Dialog
The edit channel dialog SHALL be titled "Edit <channel name> settings" and show the "Video quality" control and a "Video limit" field (whole numbers from 1 to 1000) prefilled with the channel's current values. Saving SHALL send only the changed settings, close the dialog and refresh the library. When the video limit changed, saving SHALL also trigger a sync of the channel. Cancelling or saving with nothing changed SHALL send nothing.

#### Scenario: Dialog prefilled with current settings
- **WHEN** a user chooses "Edit settings" for a channel named "Veritasium" with quality `mid` and video limit 5
- **THEN** the "Edit Veritasium settings" dialog opens with "Video quality" set to Mid and "Video limit" set to 5

#### Scenario: Saving a new quality
- **WHEN** a user changes only the video quality and saves
- **THEN** the application sends the new quality alone, closes the dialog, and does not trigger a sync

#### Scenario: Saving a new video limit
- **WHEN** a user changes the video limit and saves
- **THEN** the application sends the new video limit, closes the dialog, and triggers a sync of the channel

#### Scenario: Sync after saving fails
- **WHEN** a user saves a new video limit and the sync it triggers fails
- **THEN** the application tells the user the sync failed, and the new settings stay saved

#### Scenario: Saving with nothing changed
- **WHEN** a user saves without changing either setting
- **THEN** the application sends no request and closes the dialog

#### Scenario: Invalid video limit
- **WHEN** a user enters a video limit below 1 or above 1000
- **THEN** the dialog flags the field as invalid and does not send the request

#### Scenario: Save fails
- **WHEN** saving the settings fails
- **THEN** the dialog stays open, shows the error, and the channel keeps its previous settings

## ADDED Requirements

### Requirement: Changing The Quality Notes It Applies To New Videos Only
While the edit channel dialog's selected video quality differs from the channel's current quality, the dialog SHALL show an informational (non-warning) note reading "The new quality applies to new videos only. Videos already downloaded keep their current quality."

#### Scenario: Changing the quality
- **WHEN** a user changes a channel's video quality from High to Low in the edit channel dialog
- **THEN** the dialog notes that the new quality applies to new videos only and that downloaded videos keep their current quality

#### Scenario: Quality unchanged
- **WHEN** the selected video quality equals the channel's current quality
- **THEN** no quality note is shown
