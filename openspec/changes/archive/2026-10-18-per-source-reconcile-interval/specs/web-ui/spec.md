## ADDED Requirements

### Requirement: Reconcile Interval Field
Wherever the application lets the user set how often a channel or playlist is checked, it SHALL show a field labeled "Check for new videos every" made of a whole-number input and a unit selector offering "minutes" and "hours". The value sent SHALL be the entered number converted to minutes. When showing a stored interval, the field SHALL use hours when the interval is a whole number of hours, and minutes otherwise. An entry that is empty, not a whole number, below 1 minute or above 24 hours SHALL be flagged as invalid, and the form SHALL NOT be submitted.

#### Scenario: Interval shown in hours
- **WHEN** the field shows a stored interval of 180 minutes
- **THEN** it shows 3 with the unit "hours"

#### Scenario: Interval shown in minutes
- **WHEN** the field shows a stored interval of 90 minutes
- **THEN** it shows 90 with the unit "minutes"

#### Scenario: Entering hours
- **WHEN** a user enters 6 with the unit "hours" and submits
- **THEN** the application sends an interval of 360 minutes

#### Scenario: Invalid interval
- **WHEN** a user enters 0, a fractional number, more than 24 hours, or nothing in the field
- **THEN** the field is flagged as invalid and the form is not submitted

### Requirement: Edit Playlist Dialog
The edit playlist dialog SHALL be titled "Edit <playlist name> settings" and show the "Check for new videos every" field prefilled with the playlist's current reconcile interval. Saving SHALL send the changed interval, close the dialog and refresh the library, and SHALL NOT trigger a sync. Cancelling or saving with nothing changed SHALL send nothing.

#### Scenario: Dialog prefilled with the current interval
- **WHEN** a user chooses "Edit settings" for a playlist named "Lofi beats" whose interval is 180 minutes
- **THEN** the "Edit Lofi beats settings" dialog opens with the field showing 3 hours

#### Scenario: Saving a new interval
- **WHEN** a user changes the interval to 30 minutes and saves
- **THEN** the application sends the interval of 30 minutes alone, closes the dialog, and does not trigger a sync

#### Scenario: Saving with nothing changed
- **WHEN** a user saves without changing the interval
- **THEN** the application sends no request and closes the dialog

#### Scenario: Save fails
- **WHEN** saving the interval fails
- **THEN** the dialog stays open, shows the error, and the playlist keeps its previous interval

## MODIFIED Requirements

### Requirement: Sidebar Row Actions
Each tracked channel or playlist row in the sidebar SHALL provide an always-visible menu control that does not depend on hover. The menu SHALL offer an action to trigger a sync (reconcile), an "Edit settings" action, and an action to delete the entry. "Edit settings" SHALL open the edit channel dialog for a channel row and the edit playlist dialog for a playlist row. For channel rows it SHALL also offer an action to mark the channel watched. For playlist rows it SHALL also offer an action labeled "Exclude from home" when the playlist is shown on home, or "Include in home" when it is excluded, which changes that setting without confirmation and refreshes the home view's sections. When changing the setting fails, the application SHALL tell the user and leave the playlist as it was. The row's unwatched badge SHALL sit at the row's trailing edge, next to the menu control, with no reserved blank space between them. Deleting SHALL require the user to confirm before it takes effect. Marking a channel watched SHALL take effect without confirmation. Deleting the channel or playlist whose detail view is currently open SHALL return the user to the home view.

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

#### Scenario: Editing a playlist's settings from the sidebar
- **WHEN** a user chooses "Edit settings" from a playlist row's menu
- **THEN** the edit playlist dialog opens for that playlist, and saving it behaves as when opened from the playlist's page header

#### Scenario: Channel row menu order
- **WHEN** a user opens a channel row's menu
- **THEN** it offers, in order, sync, mark-watched, "Edit settings" and delete

#### Scenario: Playlist rows have no mark-watched action
- **WHEN** a user opens a playlist row's menu
- **THEN** it offers, in order, sync, the exclude or include in home action, "Edit settings" and delete, and no mark-watched action

#### Scenario: Excluding a playlist from home from the sidebar
- **WHEN** a user chooses "Exclude from home" from the menu of a playlist shown on home
- **THEN** the playlist's videos leave the home view, and the row's menu now offers "Include in home"

#### Scenario: Including a playlist in home from the sidebar
- **WHEN** a user chooses "Include in home" from the menu of a playlist excluded from home
- **THEN** the playlist's videos appear in the home view's sections they qualify for, and the row's menu now offers "Exclude from home"

#### Scenario: Channel rows have no home exclusion action
- **WHEN** a user opens a channel row's menu
- **THEN** it offers neither "Exclude from home" nor "Include in home"

### Requirement: Detail View Page Header
A playlist or channel detail view SHALL open with a page header showing the playlist's or channel's name, with the channel's avatar beside it for channels. Below the name it SHALL show a summary of the number of videos and, when above 0, the number of unwatched videos. The header SHALL provide a visible sync (reconcile) control and, beside it, an always-visible vertical "⋮" control opening an actions menu. The menu SHALL offer, for channels, an action to mark the channel watched and an "Edit settings" action that opens the edit channel dialog; for playlists, an action labeled "Exclude from home" when the playlist is shown on home or "Include in home" when it is excluded, which changes that setting without confirmation, and an "Edit settings" action that opens the edit playlist dialog; and, for both, last and set apart from the others, an action to delete the entry. Deleting SHALL require confirmation and, once confirmed, SHALL return the user to the home view. When changing the home setting fails, the application SHALL tell the user and leave the playlist as it was. On viewports narrower than the small breakpoint, the sync control SHALL show as an icon only, keeping its accessible label.

#### Scenario: Channel page header
- **WHEN** a user opens a channel's detail view
- **THEN** the page header shows the channel's avatar, its name, a video count summary, a sync control, and a "⋮" menu offering mark-watched, "Edit settings" and delete

#### Scenario: Playlist page header
- **WHEN** a user opens a playlist's detail view
- **THEN** the page header shows the playlist's name, a video count summary, a sync control, and a "⋮" menu offering the exclude or include in home action, "Edit settings" and delete, with no avatar and no mark-watched action

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

### Requirement: Edit Channel Dialog
The edit channel dialog SHALL be titled "Edit <channel name> settings" and show the "Video quality" control, a "Video limit" field (whole numbers from 1 to 1000) and the "Check for new videos every" field, prefilled with the channel's current values. Saving SHALL send only the changed settings, close the dialog and refresh the library. When the video limit changed, saving SHALL also trigger a sync of the channel; a changed quality or interval alone SHALL NOT. Cancelling or saving with nothing changed SHALL send nothing.

#### Scenario: Dialog prefilled with current settings
- **WHEN** a user chooses "Edit settings" for a channel named "Veritasium" with quality `mid`, video limit 5 and an interval of 180 minutes
- **THEN** the "Edit Veritasium settings" dialog opens with "Video quality" set to Mid, "Video limit" set to 5 and the interval showing 3 hours

#### Scenario: Saving a new quality
- **WHEN** a user changes only the video quality and saves
- **THEN** the application sends the new quality alone, closes the dialog, and does not trigger a sync

#### Scenario: Saving a new video limit
- **WHEN** a user changes the video limit and saves
- **THEN** the application sends the new video limit, closes the dialog, and triggers a sync of the channel

#### Scenario: Saving a new interval
- **WHEN** a user changes only the interval to 30 minutes and saves
- **THEN** the application sends the interval of 30 minutes alone, closes the dialog, and does not trigger a sync

#### Scenario: Sync after saving fails
- **WHEN** a user saves a new video limit and the sync it triggers fails
- **THEN** the application tells the user the sync failed, and the new settings stay saved

#### Scenario: Saving with nothing changed
- **WHEN** a user saves without changing any setting
- **THEN** the application sends no request and closes the dialog

#### Scenario: Invalid video limit
- **WHEN** a user enters a video limit below 1 or above 1000
- **THEN** the dialog flags the field as invalid and does not send the request

#### Scenario: Save fails
- **WHEN** saving the settings fails
- **THEN** the dialog stays open, shows the error, and the channel keeps its previous settings

### Requirement: Add Dialog Download Options
Both add dialogs SHALL present video quality, the folder name and the "Check for new videos every" field inside a collapsed "Advanced options" section that is not expanded by default; the add channel dialog SHALL also present the video limit there, and the add playlist dialog SHALL also present an "Exclude from home" checkbox there, unchecked by default, whose value is sent as the playlist's `exclude_from_home` setting. The checkbox SHALL show always-visible hint text below its label reading "Videos from this playlist won't show up in home recommendations.", exposed as the checkbox's accessible description rather than behind a tooltip. The parent folder SHALL NOT be inside "Advanced options"; it is chosen through the always-visible "Save to" list. The video quality control SHALL be labeled "Video quality" and SHALL offer a tooltip explaining that it controls the download resolution and that a lower resolution reduces storage use. In the add channel dialog, the video limit field SHALL default to 3 and SHALL accept whole numbers from 1 to 1000. In both add dialogs, the interval field SHALL default to 3 hours, and its value SHALL be sent as the new source's `reconcile_interval_minutes`.

#### Scenario: Advanced options start collapsed
- **WHEN** a user opens either add dialog
- **THEN** the video quality control, the folder name, the interval field and, in the add channel dialog, the video limit field are hidden inside a collapsed "Advanced options" section

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

#### Scenario: Interval default
- **WHEN** a user adds a channel or playlist without changing the interval field
- **THEN** it is created with `reconcile_interval_minutes` 180

#### Scenario: Adding with a custom interval
- **WHEN** a user sets the interval to 30 minutes in either add dialog and submits
- **THEN** the channel or playlist is created with `reconcile_interval_minutes` 30

#### Scenario: Exclude from home defaults to unchecked
- **WHEN** a user adds a playlist without touching the "Exclude from home" checkbox
- **THEN** the playlist is created with `exclude_from_home` `false`

#### Scenario: Adding a playlist excluded from home
- **WHEN** a user checks "Exclude from home" in the add playlist dialog and submits
- **THEN** the playlist is created with `exclude_from_home` `true`

#### Scenario: Exclude from home is explained
- **WHEN** a user expands "Advanced options" in the add playlist dialog
- **THEN** the "Exclude from home" checkbox shows the hint "Videos from this playlist won't show up in home recommendations." below its label, and assistive technology announces it as the checkbox's description

#### Scenario: Add channel dialog has no exclude from home option
- **WHEN** a user opens the add channel dialog and expands "Advanced options"
- **THEN** no "Exclude from home" checkbox is shown
