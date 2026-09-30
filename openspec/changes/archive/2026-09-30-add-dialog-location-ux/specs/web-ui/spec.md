# web-ui Delta

## ADDED Requirements

### Requirement: Add Dialog Save To Suggestions
Both add dialogs SHALL show a "Save to" control directly below the ID/URL field, visible from the moment the dialog opens, whether or not anything has been entered. It SHALL present a single-choice list of candidate parent folders, exactly one of which is selected at any time:

- the mode's default parent (`playlists/` in the add playlist dialog, `channels/` in the add channel dialog), always present, even when no tracked item is stored under it;
- the parent folders of the storage locations of existing tracked items of the dialog's own kind (tracked playlists in the add playlist dialog, tracked channels in the add channel dialog), each showing its path and how many tracked items it contains, ordered by most recent use (the parent containing the most recently added tracked item first);
- a final "Choose another folder…" entry that reveals the parent folder browser.

The list SHALL show at most 5 candidate folders (the default parent plus up to 4 derived parents) besides the "Choose another folder…" entry; when more derived parents exist, the most recently used ones are shown. Selecting a candidate SHALL make it the parent folder, and the selected candidate SHALL be visibly marked.

The parent folder SHALL only ever be set by selecting a candidate or through the parent folder browser; it SHALL NOT be free-text.

#### Scenario: Save to list visible on open
- **WHEN** a user opens either add dialog without entering anything
- **THEN** the "Save to" list is visible, with a candidate already selected

#### Scenario: One click to a previously used folder
- **WHEN** playlists are tracked at `playlists/kids/contes` and `playlists/kids/fa-la-la`, and a user selects the `playlists/kids` candidate in the add playlist dialog
- **THEN** the parent folder becomes `playlists/kids` in that single action, and the destination notice updates to match

#### Scenario: Candidate shows its occupant count
- **WHEN** the `playlists/kids` candidate is derived from 2 tracked playlists stored under it
- **THEN** the candidate identifies `playlists/kids` and shows that it contains 2 tracked items

#### Scenario: Default parent offered with nothing under it
- **WHEN** no tracked playlist is stored directly under `playlists/`
- **THEN** the add playlist dialog still offers `playlists/` as a candidate

#### Scenario: More derived parents than the list shows
- **WHEN** tracked playlists are stored under more distinct parent folders than the list's cap
- **THEN** the list shows the most recently used parents up to the cap, followed by "Choose another folder…"

#### Scenario: Suggestions come from the dialog's own kind
- **WHEN** tracked playlists exist under `playlists/kids` but no tracked channel is stored under it
- **THEN** the add channel dialog does not offer `playlists/kids` as a derived candidate

#### Scenario: Fresh install
- **WHEN** no playlists or channels are tracked
- **THEN** each dialog's list offers only the mode's default parent and "Choose another folder…"

### Requirement: Remembered Save To Parent
The application SHALL remember, per browser and per dialog mode, the parent folder last submitted successfully from that dialog, and SHALL preselect it in the "Save to" list the next time the dialog opens. A remembered parent that is no longer among the list's candidates SHALL be ignored in favor of the mode's default parent. When the remembered state cannot be read or written, the dialog SHALL fall back to the default parent without an error.

#### Scenario: Repeat add into the same folder
- **WHEN** a user adds a playlist into `playlists/kids` and later reopens the add playlist dialog
- **THEN** `playlists/kids` is preselected, and submitting requires no location action at all

#### Scenario: Remembered parent per mode
- **WHEN** a user adds a playlist into `playlists/kids` and then opens the add channel dialog
- **THEN** the add channel dialog's preselection is unaffected by the playlist dialog's remembered parent

#### Scenario: Stale remembered parent
- **WHEN** the remembered parent no longer appears among the list's candidates
- **THEN** the mode's default parent is preselected instead, without an error

#### Scenario: Storage unavailable
- **WHEN** the browser's stored state cannot be read
- **THEN** the dialog opens with the mode's default parent preselected, without an error

### Requirement: Add Dialog Folder Name
Both add dialogs SHALL offer a folder name for the directory the videos are stored in, inside the "Advanced options" section. The folder name SHALL be a single path segment: the dialog SHALL reject a folder name containing `/` and SHALL reject an empty folder name, and SHALL NOT submit the request in either case.

The folder name SHALL be pre-filled automatically rather than left blank: in the add playlist dialog, from a filesystem-safe slug of the playlist's YouTube title once the playlist has been looked up; in the add channel dialog, from a filesystem-safe slug of the entered channel handle or URL. It SHALL keep recomputing as that source changes. Once the user edits the folder name directly, the system SHALL stop overwriting it with further automatic updates for the remainder of that dialog session.

The add playlist dialog SHALL NOT ask for a playlist name. The playlist is named after its YouTube title, and the folder name determines only the directory on disk.

#### Scenario: Folder name lives in advanced options
- **WHEN** a user opens either add dialog
- **THEN** the folder name field is hidden inside the collapsed "Advanced options" section

#### Scenario: No playlist name field
- **WHEN** a user opens the add playlist dialog
- **THEN** the dialog offers no field for the playlist's name

#### Scenario: Folder name pre-fills from the playlist's YouTube title
- **WHEN** a user enters the ID or URL of a playlist titled "Lofi Beats: Study Mix" in the add playlist dialog, without having edited the folder name field
- **THEN** once the playlist has been looked up, the folder name's value is `lofi-beats-study-mix`

#### Scenario: Folder name pre-fills from the channel handle or URL
- **WHEN** a user types a channel handle or URL in the add channel dialog, without having edited the folder name field
- **THEN** the folder name's value updates to a filesystem-safe slug of that handle

#### Scenario: Manual folder name edit stops further auto-fill
- **WHEN** a user types directly into the folder name field, and then enters a different playlist ID or URL, or continues editing the channel handle/URL field
- **THEN** the folder name's value no longer changes in response to those edits

#### Scenario: Folder name containing a path separator
- **WHEN** a user enters a folder name containing `/`
- **THEN** the dialog reports the folder name as invalid and does not submit the request

#### Scenario: Empty folder name
- **WHEN** a user leaves the folder name empty
- **THEN** the dialog reports the folder name as invalid and does not submit the request

### Requirement: Add Dialog Content Fit
Each add dialog SHALL show all of its content within the dialog's bounds, without horizontal clipping or horizontal scrolling, on any viewport at least as wide as the dialog itself — including with "Advanced options" expanded and with the parent folder browser open listing occupied folders with long names. A value too long for its row SHALL truncate within the row rather than widen the dialog or clip its neighbors.

#### Scenario: Browser open with long occupied labels
- **WHEN** a user opens the parent folder browser on a folder whose entries have long names and long "in use by" labels
- **THEN** every row stays inside the dialog's bounds, truncating its text as needed, and no content is clipped by the dialog's edge

#### Scenario: Expanded dialog on a desktop viewport
- **WHEN** a user expands "Advanced options" and the folder browser on a desktop-width viewport
- **THEN** the dialog shows all controls fully inside its bounds without a horizontal scrollbar

### Requirement: Add Playlist Lookup Notice
The add playlist dialog SHALL look the entered playlist up on YouTube once the playlist ID or URL field stops changing, and SHALL show a notice between the "Save to" list and the "Advanced options" section. No notice SHALL be shown while the field is empty. While the lookup is in progress the notice SHALL say that the playlist is being looked up. A lookup answered for a value the field no longer holds SHALL be ignored.

Once the playlist has been found, the notice SHALL state how many videos YouTube reports for the playlist, the playlist's YouTube title, and the absolute destination they will be downloaded to (the videos root, parent folder and folder name joined together). It SHALL update as the parent folder or folder name changes. The notice SHALL NOT offer a "change" action; the location is changed through the "Save to" list above it.

The notice SHALL instead be shown as an error, and the dialog SHALL NOT submit the request, when:

- the value is not a playlist ID or URL, or the playlist does not exist or is not accessible on YouTube, stating which;
- YouTube could not be reached, saying so;
- the playlist is already tracked, stating that it has already been added and the name it was added under;
- the destination is already the storage location of another playlist or channel, naming the destination and the playlist or channel occupying it.

When more than one applies, the first in this list SHALL be shown. The dialog SHALL NOT submit the request until the playlist has been found.

The add playlist dialog SHALL NOT show a separate destination preview or report which directories will be created or already exist.

#### Scenario: Notice hidden before a playlist is entered
- **WHEN** a user opens the add playlist dialog and the playlist ID or URL field is empty
- **THEN** no notice is shown and the dialog does not submit

#### Scenario: Notice states the video count, title and destination
- **WHEN** a user enters the ID of a playlist titled "Lofi beats" with 42 videos, without changing the selected parent or any advanced option
- **THEN** once looked up, the notice states that the 42 videos from "Lofi beats" will be downloaded to the videos root joined with `playlists/lofi-beats`

#### Scenario: Notice follows the location
- **WHEN** a user selects a different "Save to" candidate or changes the folder name
- **THEN** the notice's destination updates to match

#### Scenario: Notice offers no change action
- **WHEN** the notice is shown, as information or as a destination-in-use error
- **THEN** it contains no "change" action

#### Scenario: Value that is not a playlist
- **WHEN** a user enters a YouTube video URL without a `list` parameter
- **THEN** the notice is shown as an error explaining that the value is not a playlist, and the dialog does not submit the request

#### Scenario: Playlist not found
- **WHEN** a user enters the ID of a playlist that does not exist or is private
- **THEN** the notice is shown as an error stating that the playlist was not found, and the dialog does not submit the request

#### Scenario: Playlist already added
- **WHEN** a user enters the ID or URL of a playlist that is already tracked under the name "Lofi beats"
- **THEN** the notice is shown as an error stating that it was already added as "Lofi beats", and the dialog does not submit the request

#### Scenario: Destination already in use
- **WHEN** the composed destination is the storage location of another playlist or channel
- **THEN** the notice is shown as an error naming the destination and the playlist or channel occupying it, and the dialog does not submit the request

#### Scenario: Created playlist is named after its YouTube title
- **WHEN** a user submits the dialog for a playlist titled "Lofi beats"
- **THEN** the playlist appears in the sidebar as "Lofi beats"

### Requirement: Add Channel Lookup Notice
The add channel dialog SHALL look the entered channel up on YouTube once the channel handle or URL field stops changing, and SHALL show a notice between the "Save to" list and the "Advanced options" section. No notice SHALL be shown while the field is empty. While the lookup is in progress the notice SHALL say that the channel is being looked up. A lookup answered for a value the field no longer holds SHALL be ignored.

Once the channel has been found, the notice SHALL show the channel's avatar, when YouTube reports one, and state how many of the channel's latest videos will be downloaded (the video limit), the channel's YouTube title, and the absolute destination they will be downloaded to (the videos root, parent folder and folder name joined together). It SHALL update as the video limit, parent folder or folder name changes. The notice SHALL NOT offer a "change" action; the location is changed through the "Save to" list above it. The folder name continues to derive from the entered handle, not from the title.

The notice SHALL instead be shown as an error, and the dialog SHALL NOT submit the request, when:

- the value is not a channel handle or channel URL, or the channel does not exist or is not accessible on YouTube, stating which;
- YouTube could not be reached, saying so;
- the channel is already tracked, stating that it has already been added and the name it was added under;
- the destination is already the storage location of another playlist or channel, naming the destination and the playlist or channel occupying it.

When more than one applies, the first in this list SHALL be shown. The dialog SHALL NOT submit the request until the channel has been found.

The add channel dialog SHALL NOT show a separate destination preview or report which directories will be created or already exist.

#### Scenario: Notice hidden before a handle is entered
- **WHEN** a user opens the add channel dialog and the channel handle or URL field is empty
- **THEN** no notice is shown and the dialog does not submit

#### Scenario: Notice states the video limit and destination
- **WHEN** a user enters `@veritasium`, the handle of a channel titled "Veritasium", without changing the selected parent or any advanced option
- **THEN** once looked up, the notice shows the channel's avatar and states that the latest 3 videos from "Veritasium" will be downloaded to the videos root joined with `channels/veritasium`

#### Scenario: Channel without an avatar
- **WHEN** the entered channel has no avatar on YouTube
- **THEN** the notice shows a placeholder in the avatar's place and otherwise reads the same

#### Scenario: Notice follows the location and options
- **WHEN** a user changes the video limit, selects a different "Save to" candidate or changes the folder name
- **THEN** the notice's video count and destination update to match

#### Scenario: Notice offers no change action
- **WHEN** the notice is shown, as information or as a destination-in-use error
- **THEN** it contains no "change" action

#### Scenario: Value that is not a channel
- **WHEN** a user enters a value that is not a handle or channel URL, such as a handle without its leading `@`
- **THEN** the notice is shown as an error explaining why the value is not a channel, and the dialog does not submit the request

#### Scenario: Channel not found
- **WHEN** a user enters the handle of a channel that does not exist or is not accessible
- **THEN** the notice is shown as an error stating that the channel was not found, and the dialog does not submit the request

#### Scenario: Channel already added
- **WHEN** a user enters the handle or URL of a channel that is already tracked under the name "Veritasium"
- **THEN** the notice is shown as an error stating that it was already added as "Veritasium", and the dialog does not submit the request

#### Scenario: Destination already in use
- **WHEN** the composed destination is the storage location of another playlist or channel
- **THEN** the notice is shown as an error naming the destination and the playlist or channel occupying it, and the dialog does not submit the request

## MODIFIED Requirements

### Requirement: Add Dialog Parent Folder Browsing
Both add dialogs SHALL let the user change the parent folder by browsing the directories under the videos root. The browser SHALL NOT be expanded by default; it SHALL be revealed by the "Save to" list's "Choose another folder…" entry, and SHALL open on the currently selected parent folder.

While the browser is open, the location being browsed SHALL be the selected parent folder, and the destination SHALL follow it. The browser SHALL offer a control that closes it, after which the chosen folder SHALL appear, selected, as a candidate in the "Save to" list for the remainder of that dialog session.

The browser SHALL list the immediate subdirectories of the location being browsed, so that the user can see what a parent already contains before naming a new directory inside it. Each listed directory that is already the storage location of an existing playlist or channel SHALL be identified as such, and SHALL name the playlist or channel that occupies it.

The browser SHALL display the location being browsed as a path whose every ancestor, including the videos root, is directly selectable, so that reaching an ancestor takes one action regardless of depth. Selecting a listed subdirectory SHALL descend into it.

The browser SHALL let the user name a directory that does not exist yet and adopt it as the parent, so that a new location can be established without creating it on the underlying storage by hand. Such a directory SHALL NOT be created at that moment; it is created together with the rest of the destination when the first video is downloaded, so abandoning the dialog leaves nothing behind on disk. While a parent that does not exist yet is selected, the browser SHALL show it as containing nothing rather than reporting an error. If the name given in the create-folder step matches a directory that already exists at that location, the browser SHALL descend into that existing directory instead of treating it as new.

#### Scenario: Revealing the browser
- **WHEN** a user activates "Choose another folder…" in either add dialog's "Save to" list
- **THEN** the browser opens on the currently selected parent folder and lists its immediate subdirectories

#### Scenario: Descending into a directory
- **WHEN** a user selects a listed directory in the browser
- **THEN** that directory becomes the parent folder and the browser lists its immediate subdirectories

#### Scenario: Closing the browser keeps the chosen folder
- **WHEN** a user browses to `playlists/kids/specials` and closes the browser
- **THEN** the "Save to" list shows `playlists/kids/specials` as its selected candidate

#### Scenario: Jumping to an ancestor
- **WHEN** a user is browsing a directory several levels below the videos root and selects one of its ancestors in the displayed path
- **THEN** that ancestor becomes the parent folder in a single action, and the browser lists its immediate subdirectories

#### Scenario: Occupied directories are identified
- **WHEN** the browser lists a directory that is the storage location of an existing playlist or channel
- **THEN** that entry is marked as occupied and names the playlist or channel using it

#### Scenario: Browsing at the videos root
- **WHEN** a user browses to the videos root
- **THEN** the browser offers nothing above it to select

#### Scenario: Adopting a parent folder that does not exist yet
- **WHEN** a user names a new directory in the browser's create-folder step and adopts it as the parent folder
- **THEN** that directory becomes the parent folder, the browser shows it as containing nothing, and nothing is created on disk at that moment

#### Scenario: Create-folder step naming a directory that already exists
- **WHEN** a user names a directory in the create-folder step that already exists at the location being browsed
- **THEN** the browser descends into that existing directory rather than adopting it as a new one

### Requirement: Add Dialog Download Options
Both add dialogs SHALL present video quality and the folder name inside a collapsed "Advanced options" section that is not expanded by default; the add channel dialog SHALL also present the video limit there. The parent folder SHALL NOT be inside "Advanced options"; it is chosen through the always-visible "Save to" list. The video quality control SHALL be labeled "Video quality" and SHALL offer a tooltip explaining that it controls the download resolution and that a lower resolution reduces storage use. In the add channel dialog, the video limit field SHALL default to 3 and SHALL accept whole numbers from 1 to 1000.

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

## REMOVED Requirements

### Requirement: Add Dialog Storage Location In Advanced Options
**Reason**: The storage location is no longer an Advanced-options concern. Parent selection is replaced by the always-visible "Save to" suggestion list (`Add Dialog Save To Suggestions`), and the folder-name rules move unchanged to `Add Dialog Folder Name`.
**Migration**: UI-only change; no data or API migration.

### Requirement: Add Playlist Destination Notice
**Reason**: Replaced by `Add Playlist Lookup Notice`: the notice keeps its lookup, destination and error behavior but loses the "change" action, whose two scenarios no longer apply now that the location is chosen through the always-visible "Save to" list.
**Migration**: UI-only change; no data or API migration.

### Requirement: Add Channel Destination Notice
**Reason**: Replaced by `Add Channel Lookup Notice`: the notice keeps its lookup, destination and error behavior but loses the "change" action, whose two scenarios no longer apply now that the location is chosen through the always-visible "Save to" list.
**Migration**: UI-only change; no data or API migration.
