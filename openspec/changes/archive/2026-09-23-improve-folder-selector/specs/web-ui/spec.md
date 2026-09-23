## ADDED Requirements

### Requirement: Add Dialog Storage Location
The add dialog SHALL present the storage location in its main body, not inside "Advanced options", as two separate controls: a parent folder, and a folder name for the directory the videos are stored in.

The parent folder SHALL be set only through the parent folder browser; it SHALL NOT be free-text. Every directory in the resulting destination SHALL therefore be one the user either browsed into — and which consequently already exists — or named explicitly in the browser's create-folder step, with that location's existing subdirectories listed on screen at the time. No directory is ever brought into existence by unreviewed free-text entry.

The folder name SHALL be a single path segment. The dialog SHALL reject a folder name containing `/`, since allowing one would let unbrowsed intermediate directories back in through the field this control exists to replace. An empty folder name SHALL also be rejected.

The parent folder SHALL default to `playlists/` in playlist mode and `channels/` in channel mode. The folder name SHALL be pre-filled automatically rather than left blank: in playlist mode, from a filesystem-safe slug of the entered playlist name; in channel mode, from a filesystem-safe slug of the entered channel handle or URL. It SHALL keep recomputing as that source field changes. Once the user edits the folder name directly, the system SHALL stop overwriting it with further automatic updates for the remainder of that dialog session.

The playlist name and the folder name are distinct: the playlist name is the playlist's display name, and the folder name determines only the directory on disk.

#### Scenario: Location controls are visible without expanding advanced options
- **WHEN** a user opens the add dialog in either mode
- **THEN** the parent folder control and the folder name field are visible without expanding "Advanced options"

#### Scenario: Parent folder defaults per mode
- **WHEN** a user opens the add dialog
- **THEN** the parent folder is `playlists/` in playlist mode and `channels/` in channel mode

#### Scenario: Folder name pre-fills from the playlist name
- **WHEN** a user is in playlist mode and types a playlist name, without having edited the folder name field
- **THEN** the folder name's value updates to a filesystem-safe slug of that playlist name

#### Scenario: Folder name pre-fills from the channel handle or URL
- **WHEN** a user is in channel mode and types a channel handle or URL, without having edited the folder name field
- **THEN** the folder name's value updates to a filesystem-safe slug of that handle

#### Scenario: Manual folder name edit stops further auto-fill
- **WHEN** a user types directly into the folder name field, and then continues editing the playlist name or channel handle/URL field
- **THEN** the folder name's value no longer changes in response to those edits

#### Scenario: Folder name containing a path separator
- **WHEN** a user enters a folder name containing `/`
- **THEN** the dialog reports the folder name as invalid and does not submit the request

#### Scenario: Empty folder name
- **WHEN** a user leaves the folder name empty
- **THEN** the dialog reports the folder name as invalid and does not submit the request

### Requirement: Add Dialog Parent Folder Browsing
The add dialog SHALL let the user change the parent folder by browsing the directories under the videos root. The browser SHALL NOT be expanded by default; the dialog SHALL show the current parent folder and a control that reveals the browser. When revealed, the browser SHALL open on the current parent folder.

The browser SHALL list the immediate subdirectories of the location being browsed, so that the user can see what a parent already contains before naming a new directory inside it. Each listed directory that is already the storage location of an existing playlist or channel SHALL be identified as such, and SHALL name the playlist or channel that occupies it.

The browser SHALL display the location being browsed as a path whose every ancestor, including the videos root, is directly selectable, so that reaching an ancestor takes one action regardless of depth. Selecting a listed subdirectory SHALL descend into it.

The browser SHALL let the user name a directory that does not exist yet and adopt it as the parent, so that a new location can be established without creating it on the underlying storage by hand. Such a directory SHALL NOT be created at that moment; it is created together with the rest of the destination when the first video is downloaded, so abandoning the dialog leaves nothing behind on disk. While a parent that does not exist yet is selected, the browser SHALL show it as containing nothing rather than reporting an error. If the name given in the create-folder step matches a directory that already exists at that location, the browser SHALL descend into that existing directory instead of treating it as new.

#### Scenario: Revealing the browser
- **WHEN** a user opens the add dialog and activates the control that reveals the parent folder browser
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

### Requirement: Add Dialog Destination Preview
The add dialog SHALL display the absolute destination the videos will be downloaded to — the videos root, the parent folder, and the folder name joined together — and SHALL update it as the parent folder or folder name changes. Only this preview SHALL be labelled as the download destination; the parent folder control SHALL be labelled as the parent, so that neither can be read as the location videos are written to.

The preview SHALL name every directory in the destination that does not exist yet and will be created, not only the final one. A parent adopted through the create-folder step does not exist either, so a destination can require more than one new directory; naming them all is what makes a mistyped parent visible before submission rather than after videos have downloaded into it.

The preview SHALL distinguish these cases:

- the destination exists already, and the videos will be added to its contents;
- one or more directories in the destination do not exist and will be created, each of them named;
- the destination is already the storage location of another playlist or channel.

In the last case the dialog SHALL report the conflict and SHALL NOT submit the request, so that a location already in use is refused before submission rather than after the server rejects it.

#### Scenario: Destination preview reflects the composed path
- **WHEN** a user changes either the parent folder or the folder name
- **THEN** the displayed destination updates to the videos root, parent folder, and folder name joined together

#### Scenario: Only the folder name is new
- **WHEN** the parent folder exists and the folder name names a directory that does not exist inside it
- **THEN** the preview identifies the folder name's directory as the one that will be created

#### Scenario: A staged parent makes more than one directory new
- **WHEN** the parent folder was adopted through the create-folder step and so does not exist yet
- **THEN** the preview names both that parent and the folder name's directory as directories that will be created

#### Scenario: Destination already exists and is unoccupied
- **WHEN** the composed destination names an existing directory that is not the storage location of any playlist or channel
- **THEN** the preview identifies it as an existing directory whose contents the videos will be added to

#### Scenario: Destination is already in use
- **WHEN** the composed destination is the storage location of another playlist or channel
- **THEN** the preview reports the conflict, names the playlist or channel occupying it, and the dialog does not submit the request

### Requirement: Add Dialog Download Options
The add dialog SHALL present video quality and, in channel mode, the video limit, inside a collapsed "Advanced options" section that is not expanded by default. The video quality control SHALL be labeled "Video quality" and SHALL offer a tooltip explaining that it controls the download resolution and that a lower resolution reduces storage use. In channel mode, the video limit field SHALL default to 3.

The storage location is not part of this section; it is presented in the dialog's main body.

#### Scenario: Advanced options start collapsed
- **WHEN** a user opens the add dialog in either mode
- **THEN** the video quality control and (in channel mode) the video limit field are hidden inside a collapsed "Advanced options" section

#### Scenario: Video quality tooltip
- **WHEN** a user reveals the "Video quality" tooltip
- **THEN** it explains that the setting controls the download resolution and that choosing a lower resolution saves storage

#### Scenario: Channel video limit default
- **WHEN** a user opens the add dialog in channel mode and does not change the video limit
- **THEN** the video limit field defaults to 3

## REMOVED Requirements

### Requirement: Add Dialog Advanced Options
**Reason**: The requirement covered the storage path together with video quality and the video limit. The storage path has moved out of "Advanced options" into the dialog's main body and is now split into a browsed parent folder and a folder name, so the single requirement no longer describes one coherent part of the dialog.

**Migration**: None required of users. The quality and video-limit behavior, including the collapsed "Advanced options" section itself, carries over unchanged to `Add Dialog Download Options`. The path field's auto-fill behavior is superseded by `Add Dialog Storage Location`, where the folder name pre-fills from the same source fields and stops auto-filling once edited by hand.

### Requirement: Advanced Options Auto-Expand On Path Conflict
**Reason**: The storage location no longer lives inside "Advanced options", and a location already in use is now detected and reported before the request is submitted, so there is no post-rejection state left to recover from by expanding a collapsed section.

**Migration**: None required of users. The behavior is superseded by `Add Dialog Destination Preview`, which reports the conflict against the composed destination and blocks submission while it stands.
