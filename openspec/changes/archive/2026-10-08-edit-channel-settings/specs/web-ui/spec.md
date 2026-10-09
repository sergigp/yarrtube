## MODIFIED Requirements

### Requirement: Detail View Page Header
A playlist or channel detail view SHALL open with a page header showing the playlist's or channel's name, with the channel's avatar beside it for channels. Below the name it SHALL show a summary of the number of videos and, when above 0, the number of unwatched videos. The header SHALL provide a visible sync (reconcile) control and, beside it, an always-visible vertical "⋮" control opening an actions menu. The menu SHALL offer, for channels, an action to mark the channel watched and an "Edit settings" action that opens the edit channel dialog; for playlists, an action labeled "Exclude from home" when the playlist is shown on home or "Include in home" when it is excluded, which changes that setting without confirmation; and, for both, last and set apart from the others, an action to delete the entry. Deleting SHALL require confirmation and, once confirmed, SHALL return the user to the home view. When changing the home setting fails, the application SHALL tell the user and leave the playlist as it was. On viewports narrower than the small breakpoint, the sync control SHALL show as an icon only, keeping its accessible label.

#### Scenario: Channel page header
- **WHEN** a user opens a channel's detail view
- **THEN** the page header shows the channel's avatar, its name, a video count summary, a sync control, and a "⋮" menu offering mark-watched, "Edit settings" and delete

#### Scenario: Playlist page header
- **WHEN** a user opens a playlist's detail view
- **THEN** the page header shows the playlist's name, a video count summary, a sync control, and a "⋮" menu offering the exclude or include in home action and delete, with no avatar, no mark-watched action and no "Edit settings" action

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

## ADDED Requirements

### Requirement: Edit Channel Dialog
The edit channel dialog SHALL be titled "Edit channel", name the channel, and show the "Video quality" control and a "Video limit" field (whole numbers from 1 to 1000) prefilled with the channel's current values. Saving SHALL send only the changed settings, close the dialog and refresh the library. When the video limit changed, saving SHALL also trigger a sync of the channel. Cancelling or saving with nothing changed SHALL send nothing.

#### Scenario: Dialog prefilled with current settings
- **WHEN** a user chooses "Edit settings" for a channel with quality `mid` and video limit 5
- **THEN** the "Edit channel" dialog opens with "Video quality" set to Mid and "Video limit" set to 5

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

### Requirement: Lowering The Video Limit Warns About Deletion
While the edit channel dialog's video limit is below the channel's current limit, the dialog SHALL show an always-visible warning that the next sync deletes the channel's downloaded videos beyond the newest ones within the new limit.

#### Scenario: Lowering the limit
- **WHEN** a user lowers a channel's video limit from 10 to 3 in the edit channel dialog
- **THEN** the dialog warns that downloaded videos beyond the 3 newest will be deleted

#### Scenario: Raising the limit
- **WHEN** a user raises the video limit or leaves it unchanged
- **THEN** no deletion warning is shown
