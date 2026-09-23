# web-ui Specification

## Purpose

Serves a browsable single-page application from the daemon's own HTTP server, so tracked playlists, their videos, and in-flight tasks can be inspected from a browser without a separate deployment or a database client.

## Requirements

### Requirement: Serve The Application Shell
The system SHALL serve the single-page application's HTML page at `GET /`, without requiring authentication.

#### Scenario: Root request
- **WHEN** a client sends `GET /` to the running daemon
- **THEN** the daemon responds with HTTP status 200 and the application's HTML

### Requirement: Serve Application Assets
The system SHALL serve every static asset (script, stylesheet, and other build output) the application's HTML page references, without requiring authentication.

#### Scenario: Asset request
- **WHEN** a client requests one of the application's referenced asset files
- **THEN** the daemon responds with HTTP status 200 and that asset's content

### Requirement: Self-Contained Deployment
The system SHALL serve the application and its assets using only the running binary, without requiring any additional file, directory, or volume to be present at runtime.

#### Scenario: Fresh deployment with no extra files
- **WHEN** the daemon is started with only its binary present (no application source or build output mounted or copied alongside it)
- **THEN** requests to the application's root and its assets still succeed as in the scenarios above

### Requirement: Client-Side Routing
The application SHALL expose a distinct, navigable client-side URL for each top-level view: the home view, a playlist detail view (per playlist ID), a channel detail view (per channel ID), and the tasks view. Navigating between these views SHALL update the browser's URL and SHALL support the browser's back and forward navigation.

#### Scenario: Navigating to a detail view updates the URL
- **WHEN** a user navigates from the home view to a playlist's or channel's detail view
- **THEN** the browser's URL changes to reflect that specific playlist or channel

#### Scenario: Browser back returns to the previous view
- **WHEN** a user is on a detail view reached by navigating from the home view, and uses the browser's back action
- **THEN** the application returns to the home view

#### Scenario: Loading a detail view's URL directly
- **WHEN** a user loads a playlist's or channel's detail URL directly (e.g. via reload or a bookmark)
- **THEN** the application renders that playlist's or channel's detail view without requiring navigation through the home view first

### Requirement: Deep-Linkable Video Selection
A playlist or channel detail view's URL SHALL support identifying a specific video within it, such that loading that URL selects that video the way clicking it in the video list would.

#### Scenario: Navigating from a home video card selects that video
- **WHEN** a user clicks a video card on the home view
- **THEN** the application navigates to that video's playlist's or channel's detail view with that video already selected

### Requirement: Logo Returns To Home
Clicking the application's logo, from any view, SHALL navigate to the home view.

#### Scenario: Clicking the logo from a detail view
- **WHEN** a user is on a playlist detail view, a channel detail view, or the tasks view, and clicks the application logo
- **THEN** the application navigates to the home view

### Requirement: Persistent Header
The application header SHALL remain fixed in place (visible, non-scrolling) as the user scrolls any view, at all viewport sizes, including mobile.

#### Scenario: Scrolling a view with a tall content area
- **WHEN** a user scrolls a view whose content exceeds the visible viewport height
- **THEN** the header remains visible in place and does not move with the scrolled content

#### Scenario: Scrolling on a mobile viewport as the browser's UI chrome shows or hides
- **WHEN** a user on a mobile browser scrolls a view while the browser's own toolbar collapses or expands
- **THEN** the header remains visible in place and does not move with the scrolled content

### Requirement: Home Page Layout
The home view SHALL display recently synced videos as a grid of cards, each showing the video's thumbnail with its title below it.

#### Scenario: Home view with recent videos
- **WHEN** one or more videos have been synced
- **THEN** the home view renders them as a grid of thumbnail-and-title cards

### Requirement: Persistent Channels And Playlists Sidebar
The application SHALL display a sidebar listing the titles of tracked channels and tracked playlists, each as a separate section, visible from every top-level view rather than only the home view. When the user is on a playlist's or channel's detail view, the sidebar SHALL mark that entry as active.

#### Scenario: Sidebar lists channels and playlists
- **WHEN** one or more channels and playlists are tracked
- **THEN** the sidebar lists each tracked channel's title under a "Channels" section and each tracked playlist's title under a "Playlists" section

#### Scenario: Selecting a sidebar entry
- **WHEN** a user clicks a channel or playlist title in the sidebar
- **THEN** the application navigates to that channel's or playlist's detail view

#### Scenario: Sidebar visible from a detail view
- **WHEN** a user is on a playlist detail view, a channel detail view, or the tasks view
- **THEN** the sidebar remains visible

#### Scenario: Active sidebar entry
- **WHEN** a user is on a playlist's or channel's detail view
- **THEN** the sidebar marks that playlist's or channel's entry as active

### Requirement: Sidebar Row Actions
Each tracked channel or playlist row in the sidebar SHALL provide a visible control to trigger a sync (reconcile) and a visible control to delete it. Deleting SHALL require the user to confirm before it takes effect. Deleting the channel or playlist whose detail view is currently open SHALL return the user to the home view.

#### Scenario: Syncing from the sidebar
- **WHEN** a user activates a sidebar row's sync control
- **THEN** the application triggers a reconcile for that channel or playlist

#### Scenario: Deleting from the sidebar requires confirmation
- **WHEN** a user activates a sidebar row's delete control
- **THEN** the application prompts for confirmation before deleting that channel or playlist

#### Scenario: Deleting the currently viewed entry
- **WHEN** a user confirms deletion of the channel or playlist whose detail view is currently open
- **THEN** the application navigates to the home view

### Requirement: Collapsible Mobile Sidebar
On viewports narrower than the desktop breakpoint, the sidebar (channels and playlists navigation) SHALL be hidden by default and SHALL NOT occupy permanent layout space. The user SHALL be able to reveal it via a control in the header, and dismiss it via a close control, a backdrop tap, or selecting a navigation item within it. Regardless of how much content the sidebar holds, showing it SHALL NOT cause the document to become scrollable. On viewports at or above the desktop breakpoint, the sidebar SHALL remain a persistent, always-visible panel, unaffected by this open/closed state.

#### Scenario: Opening the sidebar on a mobile viewport
- **WHEN** a user on a mobile-width viewport activates the header's sidebar control
- **THEN** the sidebar becomes visible over the current view

#### Scenario: Dismissing the sidebar on a mobile viewport
- **WHEN** a user on a mobile-width viewport, with the sidebar open, taps the close control, taps outside the sidebar, or selects a channel or playlist in it
- **THEN** the sidebar is hidden again

#### Scenario: A long navigation list on a mobile viewport
- **WHEN** a user on a mobile-width viewport opens the sidebar and it contains enough channels and playlists that its content exceeds the viewport height
- **THEN** the sidebar's own content scrolls internally and the document does not become scrollable

#### Scenario: Desktop viewport is unaffected
- **WHEN** a user is on a viewport at or above the desktop breakpoint
- **THEN** the sidebar is always visible as a persistent side panel, regardless of the open/closed state used on mobile

### Requirement: Unified Add Dialog
The application SHALL provide a single entry point — one "Add" control, always available — that opens one dialog for adding either a playlist or a channel, with a switcher inside the dialog to choose which.

#### Scenario: Opening the add dialog
- **WHEN** a user clicks the "Add" control
- **THEN** the application opens a single dialog offering a choice between adding a playlist and adding a channel

#### Scenario: Switching between playlist and channel mode
- **WHEN** a user changes the dialog's switcher from one mode to the other
- **THEN** the dialog's fields change to match the selected mode

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

### Requirement: Add Dialog Dismissal
The add dialog SHALL NOT close in response to a click outside the dialog. It SHALL close in response to the Escape key or its explicit close control.

#### Scenario: Clicking outside the dialog
- **WHEN** the add dialog is open and a user clicks outside it
- **THEN** the dialog remains open

#### Scenario: Closing via Escape or the close control
- **WHEN** the add dialog is open and a user presses Escape or activates its close control
- **THEN** the dialog closes

### Requirement: Detail View Fixed Video Area
In a playlist or channel detail view, the video player and the selected video's detail pane SHALL remain fixed in place as the user scrolls; only the video list SHALL scroll. This SHALL hold at all viewport sizes, including mobile.

#### Scenario: Scrolling the video list
- **WHEN** a user scrolls the video list in a playlist or channel detail view
- **THEN** the video player and the video detail pane do not move

#### Scenario: Scrolling the video list on a mobile viewport
- **WHEN** a user on a mobile-width viewport scrolls the video list in a playlist or channel detail view
- **THEN** the video player and the video detail pane do not move

### Requirement: Video Detail Links To YouTube
A selected video's detail pane SHALL include a link that opens that video on YouTube in a new tab.

#### Scenario: Opening a video on YouTube
- **WHEN** a user activates the "Open on YouTube" link for a selected video
- **THEN** a new tab opens to that video's page on youtube.com
