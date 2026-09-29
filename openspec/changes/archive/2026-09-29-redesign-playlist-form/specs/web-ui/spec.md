## MODIFIED Requirements

### Requirement: Add Dialog Parent Folder Browsing
Both add dialogs SHALL let the user change the parent folder by browsing the directories under the videos root. The browser SHALL NOT be expanded by default; the dialog SHALL show the current parent folder and a control that reveals the browser, both inside "Advanced options". When revealed, the browser SHALL open on the current parent folder.

The browser SHALL list the immediate subdirectories of the location being browsed, so that the user can see what a parent already contains before naming a new directory inside it. Each listed directory that is already the storage location of an existing playlist or channel SHALL be identified as such, and SHALL name the playlist or channel that occupies it.

The browser SHALL display the location being browsed as a path whose every ancestor, including the videos root, is directly selectable, so that reaching an ancestor takes one action regardless of depth. Selecting a listed subdirectory SHALL descend into it.

The browser SHALL let the user name a directory that does not exist yet and adopt it as the parent, so that a new location can be established without creating it on the underlying storage by hand. Such a directory SHALL NOT be created at that moment; it is created together with the rest of the destination when the first video is downloaded, so abandoning the dialog leaves nothing behind on disk. While a parent that does not exist yet is selected, the browser SHALL show it as containing nothing rather than reporting an error. If the name given in the create-folder step matches a directory that already exists at that location, the browser SHALL descend into that existing directory instead of treating it as new.

#### Scenario: Revealing the browser
- **WHEN** a user activates the control that reveals the parent folder browser in either add dialog
- **THEN** the browser opens on the current parent folder and lists its immediate subdirectories

#### Scenario: Descending into a directory
- **WHEN** a user selects a listed directory in the browser
- **THEN** that directory becomes the parent folder and the browser lists its immediate subdirectories

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
Both add dialogs SHALL present video quality and the storage location inside a collapsed "Advanced options" section that is not expanded by default; the add channel dialog SHALL also present the video limit there. The video quality control SHALL be labeled "Video quality" and SHALL offer a tooltip explaining that it controls the download resolution and that a lower resolution reduces storage use. In the add channel dialog, the video limit field SHALL default to 3 and SHALL accept whole numbers from 1 to 1000.

#### Scenario: Advanced options start collapsed
- **WHEN** a user opens either add dialog
- **THEN** the video quality control, the storage location and, in the add channel dialog, the video limit field are hidden inside a collapsed "Advanced options" section

#### Scenario: Video quality tooltip
- **WHEN** a user reveals the "Video quality" tooltip
- **THEN** it explains that the setting controls the download resolution and that choosing a lower resolution saves storage

#### Scenario: Channel video limit default
- **WHEN** a user opens the add channel dialog and does not change the video limit
- **THEN** the video limit field defaults to 3

#### Scenario: Channel video limit range
- **WHEN** a user enters a video limit below 1 or above 1000 in the add channel dialog
- **THEN** the dialog flags the field as invalid and does not submit the request

## ADDED Requirements

### Requirement: Add Dialog Storage Location In Advanced Options
Both add dialogs SHALL present the storage location inside their collapsed "Advanced options" section. In both, the location SHALL consist of two separate controls: a parent folder, and a folder name for the directory the videos are stored in.

The parent folder SHALL be set only through the parent folder browser; it SHALL NOT be free-text. Every directory in the resulting destination SHALL therefore be one the user either browsed into — and which consequently already exists — or named explicitly in the browser's create-folder step, with that location's existing subdirectories listed on screen at the time. No directory is ever brought into existence by unreviewed free-text entry.

The folder name SHALL be a single path segment. The dialog SHALL reject a folder name containing `/`, since allowing one would let unbrowsed intermediate directories back in through the field this control exists to replace. An empty folder name SHALL also be rejected.

The parent folder SHALL default to `playlists/` in the add playlist dialog and `channels/` in the add channel dialog. The folder name SHALL be pre-filled automatically rather than left blank: in the add playlist dialog, from a filesystem-safe slug of the playlist's YouTube title once the playlist has been looked up; in the add channel dialog, from a filesystem-safe slug of the entered channel handle or URL. It SHALL keep recomputing as that source changes. Once the user edits the folder name directly, the system SHALL stop overwriting it with further automatic updates for the remainder of that dialog session.

The add playlist dialog SHALL NOT ask for a playlist name. The playlist is named after its YouTube title, and the folder name determines only the directory on disk.

#### Scenario: Location controls are inside advanced options
- **WHEN** a user opens either add dialog
- **THEN** the parent folder control and the folder name field are hidden inside the collapsed "Advanced options" section

#### Scenario: No playlist name field
- **WHEN** a user opens the add playlist dialog
- **THEN** the dialog offers no field for the playlist's name

#### Scenario: Parent folder defaults per mode
- **WHEN** a user opens an add dialog
- **THEN** the parent folder is `playlists/` in the add playlist dialog and `channels/` in the add channel dialog

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


### Requirement: Add Playlist Destination Notice
The add playlist dialog SHALL look the entered playlist up on YouTube once the playlist ID or URL field stops changing, and SHALL show a notice between that field and the "Advanced options" section. No notice SHALL be shown while the field is empty. While the lookup is in progress the notice SHALL say that the playlist is being looked up. A lookup answered for a value the field no longer holds SHALL be ignored.

Once the playlist has been found, the notice SHALL state how many videos YouTube reports for the playlist, the playlist's YouTube title, and the absolute destination they will be downloaded to (the videos root, parent folder and folder name joined together). It SHALL update as the parent folder or folder name changes, and SHALL offer a "change" action that expands "Advanced options".

The notice SHALL instead be shown as an error, and the dialog SHALL NOT submit the request, when:

- the value is not a playlist ID or URL, or the playlist does not exist or is not accessible on YouTube, stating which;
- YouTube could not be reached, saying so;
- the playlist is already tracked, stating that it has already been added and the name it was added under;
- the destination is already the storage location of another playlist or channel, naming the destination and the playlist or channel occupying it, and still offering the "change" action.

When more than one applies, the first in this list SHALL be shown. The dialog SHALL NOT submit the request until the playlist has been found.

The add playlist dialog SHALL NOT show a separate destination preview or report which directories will be created or already exist.

#### Scenario: Notice hidden before a playlist is entered
- **WHEN** a user opens the add playlist dialog and the playlist ID or URL field is empty
- **THEN** no notice is shown and the dialog does not submit

#### Scenario: Notice states the video count, title and destination
- **WHEN** a user enters the ID of a playlist titled "Lofi beats" with 42 videos, without changing any advanced option
- **THEN** once looked up, the notice states that the 42 videos from "Lofi beats" will be downloaded to the videos root joined with `playlists/lofi-beats`

#### Scenario: Notice follows the advanced options
- **WHEN** a user changes the parent folder or the folder name
- **THEN** the notice's destination updates to match

#### Scenario: Change action expands advanced options
- **WHEN** a user activates the notice's "change" action
- **THEN** "Advanced options" expands, showing the storage location controls

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

## REMOVED Requirements

### Requirement: Add Dialog Storage Location
**Reason**: The add playlist dialog no longer has a name field or shows its location in the main body. Both dialogs now keep the storage location in "Advanced options", and the playlist folder name follows the YouTube title. Replaced by "Add Dialog Storage Location In Advanced Options".
**Migration**: See "Add Dialog Storage Location In Advanced Options". The parent folder, folder name and validation rules are unchanged.

### Requirement: Add Dialog Destination Preview
**Reason**: The add playlist dialog now reports its destination through the Add Playlist Destination Notice, as the add channel dialog already does, so no dialog shows the destination preview box or its "will be created" / "already exists" hints.
**Migration**: The destination and a destination conflict are stated by each dialog's destination notice. Which directories will be created is no longer shown; the folder browser still lists what exists.
