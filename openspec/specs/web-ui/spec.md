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
The home view SHALL display up to three sections, in this order: "Continue watching", "Quick watches" and "Latest videos", loaded together in one request. A video SHALL appear in at most one section. "Latest videos" SHALL display up to 18 recently synced videos. "Continue watching" SHALL display up to 6 videos the user has started and not finished recently, and SHALL show on each card how far the video has been watched. "Quick watches" SHALL display up to 6 short unwatched videos. "Continue watching" and "Quick watches" SHALL be hidden when they have no videos, including while their videos are loading or when they cannot be loaded. Each section SHALL display its videos as a grid of cards, each showing the video's thumbnail with its title below it. For videos from a tracked channel, the card SHALL also show the channel's avatar and name.

#### Scenario: Home view with recent videos
- **WHEN** one or more videos have been synced
- **THEN** the home view renders them under "Latest videos" as a grid of thumbnail-and-title cards

#### Scenario: Channel video card
- **WHEN** a video in any home section comes from a tracked channel
- **THEN** its card shows the channel's avatar and name below the title

#### Scenario: Playlist video card
- **WHEN** a video in any home section comes from a tracked playlist
- **THEN** its card shows no channel avatar or name

#### Scenario: Continue watching shown first
- **WHEN** at least one video has been started and not finished recently
- **THEN** the home view shows a "Continue watching" section above the other sections, listing that video

#### Scenario: Continue watching card shows progress
- **WHEN** a video is shown under "Continue watching"
- **THEN** its thumbnail shows a progress bar proportional to its saved playback position over its duration

#### Scenario: Quick watches shown above latest videos
- **WHEN** at least one short unwatched video has been downloaded
- **THEN** the home view shows a "Quick watches" section below "Continue watching" and above "Latest videos", listing that video

#### Scenario: Sections show a bounded number of videos
- **WHEN** more videos qualify for a section than it shows
- **THEN** "Continue watching" and "Quick watches" each show at most 6 videos, and "Latest videos" shows at most 18

#### Scenario: A video appears in one section only
- **WHEN** a video qualifies for more than one home section
- **THEN** it is shown only in the first of them, in section order

#### Scenario: Empty sections are hidden
- **WHEN** no video qualifies for "Continue watching" or "Quick watches"
- **THEN** that section, including its heading, is not shown

#### Scenario: Optional sections that fail to load are hidden
- **WHEN** the videos for "Continue watching" or "Quick watches" are still loading or cannot be loaded
- **THEN** that section, including its heading, is not shown, and "Latest videos" still renders (showing the load error when there is one)

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

### Requirement: Collapsible Mobile Sidebar
On viewports narrower than the desktop breakpoint, the sidebar (channels and playlists navigation) SHALL be hidden by default and SHALL NOT occupy permanent layout space. The user SHALL be able to reveal it via a control in the header. That same header control SHALL dismiss it while it is open, SHALL indicate whether the sidebar is open (both visually, by showing a close icon while open, and to assistive technology), and SHALL be the sidebar's only close control. While the sidebar is open, the header SHALL remain visible and usable above it and its backdrop. The sidebar SHALL also be dismissed by a backdrop tap, pressing Escape, selecting a navigation item within it, activating the header's home link, or any other navigation to a different view. When open, it SHALL cover no more than about two thirds of a typical phone's width, leaving the view behind it visible. While it is open, the page behind it SHALL NOT scroll, whatever the sidebar contains. On viewports at or above the desktop breakpoint, the sidebar SHALL remain a persistent, always-visible panel, unaffected by this open/closed state.

#### Scenario: Opening the sidebar on a mobile viewport
- **WHEN** a user on a mobile-width viewport activates the header's sidebar control
- **THEN** the sidebar becomes visible over the current view, below the header
- **AND** the header control indicates the sidebar is open and offers to close it

#### Scenario: Header stays usable while the sidebar is open
- **WHEN** a user on a mobile-width viewport has the sidebar open
- **THEN** the header (home link, sidebar control, settings) remains visible and can be activated without first dismissing the sidebar

#### Scenario: Dismissing the sidebar on a mobile viewport
- **WHEN** a user on a mobile-width viewport, with the sidebar open, activates the header's sidebar control, taps outside the sidebar, presses Escape, or selects a channel or playlist in it
- **THEN** the sidebar is hidden again
- **AND** the header control indicates the sidebar is closed

#### Scenario: Navigating from the header closes the sidebar
- **WHEN** a user on a mobile-width viewport, with the sidebar open, activates the header's home link or navigates to another view from the header (e.g. Settings → Tasks)
- **THEN** the sidebar is hidden and the chosen view is shown

#### Scenario: A long navigation list on a mobile viewport
- **WHEN** a user on a mobile-width viewport opens the sidebar and it contains enough channels and playlists that its content exceeds the viewport height
- **THEN** the sidebar's own content scrolls internally and the page behind it does not scroll

#### Scenario: Scroll position survives opening the sidebar
- **WHEN** a user on a mobile-width viewport scrolls a view, opens the sidebar, and then dismisses it without navigating
- **THEN** the view is at the same scroll position as before the sidebar opened

#### Scenario: Desktop viewport is unaffected
- **WHEN** a user is on a viewport at or above the desktop breakpoint
- **THEN** the sidebar is always visible as a persistent side panel, regardless of the open/closed state used on mobile

### Requirement: Sidebar Add Entries
The sidebar's Channels section SHALL offer an "Add channel" control and its Playlists section an "Add playlist" control, each placed between the section heading and the section's list and always visible regardless of the section's rows or collapsed state. "Add channel" SHALL open the add channel dialog and "Add playlist" SHALL open the add playlist dialog. On viewports narrower than the desktop breakpoint, activating either control SHALL close the sidebar before the dialog is shown.

#### Scenario: Opening the add channel dialog from the sidebar
- **WHEN** a user activates "Add channel" in the sidebar's Channels section
- **THEN** the add channel dialog opens

#### Scenario: Opening the add playlist dialog from the sidebar
- **WHEN** a user activates "Add playlist" in the sidebar's Playlists section
- **THEN** the add playlist dialog opens

#### Scenario: Add entries with nothing tracked
- **WHEN** no channels or playlists are tracked
- **THEN** both sections still show their add control beneath their heading

#### Scenario: Adding from the mobile sidebar
- **WHEN** a user on a mobile-width viewport opens the sidebar and activates "Add channel" or "Add playlist"
- **THEN** the sidebar closes and the matching add dialog is shown

### Requirement: Header Settings Menu
The header SHALL offer, at its trailing edge, a settings control shown as a gear icon with the accessible name "Settings". Activating it SHALL open a menu whose items are navigation to other views. The menu SHALL contain a "Tasks" item that navigates to the tasks view. The header SHALL NOT show any other control for adding channels or playlists or for reaching the tasks view.

#### Scenario: Reaching the tasks view
- **WHEN** a user activates the header's settings control and chooses "Tasks"
- **THEN** the application navigates to the tasks view

#### Scenario: Header controls
- **WHEN** a user views any page
- **THEN** the header shows the settings control and no "Add" or "Tasks" button

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

### Requirement: Add Dialog Download Options
Both add dialogs SHALL present video quality and the folder name inside a collapsed "Advanced options" section that is not expanded by default; the add channel dialog SHALL also present the video limit there, and the add playlist dialog SHALL also present an "Exclude from home" checkbox there, unchecked by default, whose value is sent as the playlist's `exclude_from_home` setting. The checkbox SHALL show always-visible hint text below its label reading "Videos from this playlist won't show up in home recommendations.", exposed as the checkbox's accessible description rather than behind a tooltip. The parent folder SHALL NOT be inside "Advanced options"; it is chosen through the always-visible "Save to" list. The video quality control SHALL be labeled "Video quality" and SHALL offer a tooltip explaining that it controls the download resolution and that a lower resolution reduces storage use. In the add channel dialog, the video limit field SHALL default to 3 and SHALL accept whole numbers from 1 to 1000.

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

#### Scenario: Exclude from home is explained
- **WHEN** a user expands "Advanced options" in the add playlist dialog
- **THEN** the "Exclude from home" checkbox shows the hint "Videos from this playlist won't show up in home recommendations." below its label, and assistive technology announces it as the checkbox's description

#### Scenario: Add channel dialog has no exclude from home option
- **WHEN** a user opens the add channel dialog and expands "Advanced options"
- **THEN** no "Exclude from home" checkbox is shown

### Requirement: Add Dialog Content Fit
Each add dialog SHALL show all of its content within the dialog's bounds, without horizontal clipping or horizontal scrolling, on any viewport at least as wide as the dialog itself — including with "Advanced options" expanded and with the parent folder browser open listing occupied folders with long names. A value too long for its row SHALL truncate within the row rather than widen the dialog or clip its neighbors.

#### Scenario: Browser open with long occupied labels
- **WHEN** a user opens the parent folder browser on a folder whose entries have long names and long "in use by" labels
- **THEN** every row stays inside the dialog's bounds, truncating its text as needed, and no content is clipped by the dialog's edge

#### Scenario: Expanded dialog on a desktop viewport
- **WHEN** a user expands "Advanced options" and the folder browser on a desktop-width viewport
- **THEN** the dialog shows all controls fully inside its bounds without a horizontal scrollbar

### Requirement: Add Dialog Dismissal
Neither add dialog SHALL close in response to a click outside the dialog. Each SHALL close in response to the Escape key or its explicit close control.

#### Scenario: Clicking outside the dialog
- **WHEN** an add dialog is open and a user clicks outside it
- **THEN** the dialog remains open

#### Scenario: Closing via Escape or the close control
- **WHEN** an add dialog is open and a user presses Escape or activates its close control
- **THEN** the dialog closes

### Requirement: Detail View Fixed Video Area
In a playlist or channel detail view on viewports at or above the desktop breakpoint, the video player and the selected video's detail pane SHALL remain fixed in place as the user scrolls; only the video list SHALL scroll. On viewports narrower than the desktop breakpoint, the document itself SHALL scroll. The video player SHALL stay pinned directly below the header, and the video detail pane and video list SHALL scroll with the page beneath it.

#### Scenario: Scrolling the video list
- **WHEN** a user on a desktop-width viewport scrolls the video list in a playlist or channel detail view
- **THEN** the video player and the video detail pane do not move

#### Scenario: Scrolling the video list on a mobile viewport
- **WHEN** a user on a mobile-width viewport scrolls a playlist or channel detail view
- **THEN** the document scrolls, the video player stays pinned directly below the header, and the video detail pane and video list move up beneath it

#### Scenario: Browser toolbars on a mobile browser
- **WHEN** a user on a mobile browser with collapsible or translucent toolbars scrolls a view
- **THEN** the page content extends behind the browser's toolbars and the browser is able to collapse them, as it does for any normally scrolling page

### Requirement: Video Detail Links To YouTube
A selected video's detail pane SHALL include a link that opens that video on YouTube in a new tab.

#### Scenario: Opening a video on YouTube
- **WHEN** a user activates the "Open on YouTube" link for a selected video
- **THEN** a new tab opens to that video's page on youtube.com

### Requirement: Resume Playback
When a user plays a video in a playlist or channel detail view, the player SHALL start from the video's saved playback position if it is unwatched and has a saved position above 0. Otherwise it SHALL start from the beginning.

#### Scenario: Resuming a partly watched video
- **WHEN** a user opens an unwatched video with a saved playback position
- **THEN** playback starts from that position

#### Scenario: Opening a watched video
- **WHEN** a user opens a watched video
- **THEN** playback starts from the beginning

### Requirement: Report Playback Progress
While a video plays in a playlist or channel detail view, the application SHALL report the current playback position to the daemon at most every 15 seconds. It SHALL also report the position when playback pauses or ends, when the user switches to another video or leaves the view, and when the page is closed or hidden. Each report SHALL state whether the video was watched when the current playback session began.

#### Scenario: Progress reported while playing
- **WHEN** a video has been playing for 15 seconds or more since the last report
- **THEN** the application reports the current position

#### Scenario: Progress reported on pause
- **WHEN** a user pauses a playing video
- **THEN** the application reports the current position

#### Scenario: Progress reported on leaving
- **WHEN** a user switches video, navigates away, or closes the tab while a video is playing
- **THEN** the application reports the last position

#### Scenario: Report states the session's watched state
- **WHEN** a user opens an unwatched video and its progress is reported
- **THEN** the report states the video was unwatched, even if a refresh has since shown it watched

### Requirement: Playback Session Follows Watch State
A playlist or channel detail view's player SHALL track playback in sessions. A session SHALL begin when a video is loaded into the player, and SHALL record whether the video was watched at that moment. When the selected video's watched state changes while it is loaded, whether marked from this view, from its channel or elsewhere, the player SHALL begin a new session. It SHALL pause, return to the start of the video, and report nothing until playback moves on from there. After the user marks a video watched in the application, the selected video's watched state SHALL be refreshed promptly, without waiting for the periodic refresh.

#### Scenario: Marking the playing video watched
- **WHEN** a user marks the video that is playing as watched, then leaves the view
- **THEN** the player pauses at the start, no position from before the mark is reported, and the video stays watched

#### Scenario: Marking the channel watched while a video is loaded
- **WHEN** a user marks a channel watched while one of its partly watched videos is loaded in the player, then leaves the view
- **THEN** the video stays watched, leaves "Continue watching", and the channel's badge stays at 0

#### Scenario: Playing again after a mark
- **WHEN** a user presses play on a video after it was marked watched while loaded
- **THEN** playback starts from the beginning

### Requirement: Watched Tick On Video Thumbnails
Every video thumbnail in the playlist detail, channel detail and home views SHALL display a tick when that video is watched, and SHALL NOT display one when it is unwatched.

#### Scenario: Watched video thumbnail
- **WHEN** a watched video is listed in a playlist, channel or home view
- **THEN** its thumbnail displays a tick

#### Scenario: Unwatched video thumbnail
- **WHEN** an unwatched video is listed
- **THEN** its thumbnail does not display a tick

### Requirement: Unwatched Badge On Sidebar Channels
Each channel row in the sidebar SHALL display a badge with the channel's unwatched video count when that count is above 0, and no badge when it is 0. The badge SHALL update without a manual page reload. After an action in the application that changes a channel's unwatched count, the badge and the channel's sidebar position SHALL update promptly, without waiting for the periodic refresh. Such actions are: a video becoming watched or unwatched during playback, marking a single video watched, marking the channel watched, syncing, and adding or deleting a channel. During playback, the channel list SHALL be refetched only when a progress report changes the video's watched state.

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
- **THEN** the application does not refetch the channel list because of it

#### Scenario: Badge updates after marking watched
- **WHEN** a user marks a channel watched from its sidebar row or its detail view
- **THEN** the channel's badge disappears promptly and the channel moves into the caught-up group

#### Scenario: Badge updates after marking a video watched
- **WHEN** a user marks one of a channel's unwatched videos watched from a video menu
- **THEN** the channel's badge count decreases by one promptly without a page reload

### Requirement: Mark Channel Watched From Channel View
A channel detail view SHALL provide an action to mark the channel watched, which takes effect without confirmation. The action SHALL sit in the page header's "⋮" actions menu.

#### Scenario: Marking a channel watched from its view
- **WHEN** a user chooses the mark-watched action from the page header's "⋮" menu in a channel's detail view
- **THEN** every downloaded video in the list shows a tick, and the channel's sidebar badge disappears

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

### Requirement: Playback Speed Control
The title row of a playlist or channel detail view's video detail pane SHALL show a playback speed control next to the "⋮" actions menu, labelled with the current playback speed (e.g. "1x", "1.5x"). Activating it SHALL open a menu offering the speeds 1x, 1.1x, 1.25x, 1.5x and 2x, with the current speed marked. Choosing a speed SHALL change the playing video's speed immediately without interrupting playback. The speed SHALL apply only to the selected video: selecting another video, or reloading the application, SHALL start playback at 1x. When the speed is changed through the browser's own player controls, the control SHALL show that speed, even when it is not one of the offered speeds. The control SHALL be disabled while the selected video has not finished downloading.

#### Scenario: Choosing a faster speed
- **WHEN** a user opens the speed control of a downloaded video and chooses "1.5x"
- **THEN** the video plays at 1.5 times normal speed and the control reads "1.5x"

#### Scenario: Default speed
- **WHEN** a user selects a downloaded video
- **THEN** the speed control reads "1x" and the video plays at normal speed

#### Scenario: Speed resets when another video is selected
- **WHEN** a user plays a video at 2x and then selects another video
- **THEN** the newly selected video plays at 1x and the control reads "1x"

#### Scenario: Speed resets on reload
- **WHEN** a user plays a video at 1.25x and reloads the page
- **THEN** the video plays at 1x and the control reads "1x"

#### Scenario: Speed changed from the browser's own controls
- **WHEN** a user changes the speed to 1.75x through the browser's native player menu
- **THEN** the speed control reads "1.75x" and none of the offered speeds is marked as current

#### Scenario: Video not downloaded yet
- **WHEN** the selected video is pending, downloading or errored
- **THEN** the speed control is shown disabled

### Requirement: Channel Avatar Navigates To Channel
Wherever the application shows a channel's avatar or name, outside the sidebar row (which already links to the channel), activating it SHALL navigate to that channel's detail view. On a home card, activating the channel avatar or name SHALL navigate to the channel. Activating the card's thumbnail or title SHALL still navigate to the video.

#### Scenario: Tapping a channel avatar on a home card
- **WHEN** a user taps the channel avatar or channel name on a home view card
- **THEN** the application navigates to that channel's detail view

#### Scenario: Tapping a home card's thumbnail
- **WHEN** a user taps the thumbnail or title of a home view card
- **THEN** the application navigates to that video in its channel's or playlist's detail view

#### Scenario: Tapping the avatar in a channel's video detail pane
- **WHEN** a user taps the channel avatar in a channel detail view's video detail pane
- **THEN** the application navigates to that channel's detail view

### Requirement: Video Detail Title Prominence
In the video detail pane, the video's title SHALL occupy its own full-width row. No other elements SHALL share that row at any viewport size, apart from the channel avatar in a channel detail view.

#### Scenario: Long title on a mobile viewport
- **WHEN** a user on a mobile-width viewport selects a video with a long title
- **THEN** the title wraps across the full width of the detail pane, and the meta line appears below it

### Requirement: Collapsible Video Details
The video detail pane SHALL always show the video's title and its meta line. The rest of the pane (the status indicator when the video is not downloaded, the description, the file path and the "Open on YouTube" link) SHALL sit in a section the user can expand and collapse. This section SHALL start collapsed on viewports narrower than the desktop breakpoint and expanded at or above it. The pane SHALL NOT show a quality indicator, and SHALL NOT show a status indicator for a downloaded video.

#### Scenario: Details start collapsed on mobile
- **WHEN** a user on a mobile-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the video's title, its meta line and an expand control, and hides the description, path and YouTube link

#### Scenario: Expanding details
- **WHEN** a user activates the expand control in a collapsed detail pane
- **THEN** the description, file path and "Open on YouTube" link become visible, and the control collapses them again when activated

#### Scenario: Details start expanded on desktop
- **WHEN** a user on a desktop-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the title, meta line, description, path and YouTube link without needing to be expanded

#### Scenario: Downloaded video shows no status indicator
- **WHEN** a user expands the details of a downloaded video
- **THEN** no status or quality indicator is shown

#### Scenario: Pending video shows its status
- **WHEN** a user expands the details of a video that is pending, downloading, retrying or errored
- **THEN** a status indicator shows that state, and no quality indicator is shown

### Requirement: Video Detail Meta Line
The video detail pane SHALL show a meta line directly below the title, visible whether the details section is expanded or collapsed. The line SHALL read `<channel name> · Published <date> · Synced <relative time>`:
- The channel name SHALL appear only in a playlist detail view.
- The publish time SHALL be shown as a date, without a time of day.
- The sync time SHALL be shown relative to now (for example "2d ago"), and its full date and time SHALL be available on hover.
- Any part whose value is absent SHALL be omitted along with its separator. When every part is absent, the line SHALL not be shown.

#### Scenario: Downloaded video in a playlist view
- **WHEN** a user selects a downloaded video with generated metadata in a playlist detail view
- **THEN** the meta line shows the video's channel name, "Published" followed by its publish date, and "Synced" followed by how long ago it was synced

#### Scenario: Downloaded video in a channel view
- **WHEN** a user selects a downloaded video with generated metadata in a channel detail view
- **THEN** the meta line shows "Published" with its publish date and "Synced" with how long ago it was synced, without the channel name

#### Scenario: Meta line on a collapsed mobile pane
- **WHEN** a user on a mobile-width viewport opens a detail view with the details section collapsed
- **THEN** the meta line is visible below the title

#### Scenario: Video without metadata or sync time
- **WHEN** a user selects a pending video that has neither generated metadata nor a sync time
- **THEN** no meta line is shown

### Requirement: Video Detail Description
The expandable section of the video detail pane SHALL show the video's description, preserving its line breaks and rendering web URLs in it as links that open in a new tab. The description SHALL be clamped to about four lines, with a "Show more" control, shown only when the text exceeds the clamp, that reveals the rest and can collapse it again. When the video has no description, or an empty one, no description block SHALL be shown.

#### Scenario: Long description
- **WHEN** a user expands the details of a video whose description is longer than four lines
- **THEN** the first four lines are shown followed by a "Show more" control, and activating it reveals the whole description

#### Scenario: Short description
- **WHEN** a user expands the details of a video whose description fits in four lines
- **THEN** the whole description is shown and no "Show more" control is present

#### Scenario: Link in the description
- **WHEN** a user activates a URL inside a video's description
- **THEN** that URL opens in a new tab

#### Scenario: Video without a description
- **WHEN** a user expands the details of a video with no description
- **THEN** no description block is shown

### Requirement: Compact Mobile Player
On viewports narrower than the desktop breakpoint, the video player area SHALL be sized to the video's own aspect ratio, with no blank padding above or below the video. The player SHALL start one standard page gutter below the detail view's page header. While the page scrolls, the page header SHALL scroll away and the player SHALL stay pinned directly below the application header.

#### Scenario: Downloaded video on a mobile viewport
- **WHEN** a user on a mobile-width viewport views a downloaded 16:9 video in a detail view
- **THEN** the player area is exactly as tall as the video at full width, and it starts one page gutter below the detail view's page header

#### Scenario: Scrolling past the page header on a mobile viewport
- **WHEN** a user on a mobile-width viewport scrolls down a detail view
- **THEN** the page header scrolls out of view and the player stays pinned directly below the application header

#### Scenario: Video not downloaded on a mobile viewport
- **WHEN** a user on a mobile-width viewport selects a video that has not been downloaded
- **THEN** the placeholder message appears in an area with the same 16:9 proportions as the player

### Requirement: Application Icon And Web App Manifest
The application SHALL serve its own Yarrtube icon as the browser favicon and as the home screen icon on iOS, instead of the default template icon. It SHALL declare a theme color that matches the header background. It SHALL serve a web app manifest so that, when added to a phone's home screen, it opens in standalone mode without the browser's toolbars.

#### Scenario: Browser tab icon
- **WHEN** a user opens the application in a browser
- **THEN** the tab shows the Yarrtube icon

#### Scenario: Adding to the iOS home screen
- **WHEN** a user adds the application to their iOS home screen and launches it from there
- **THEN** the home screen shows the Yarrtube icon, and the application opens without Safari's address bar and toolbar

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

### Requirement: Sidebar Section Headings
The sidebar's "Channels" and "Playlists" section titles SHALL be styled as headings, visually distinct from the channel and playlist rows beneath them.

#### Scenario: Distinguishing section titles from rows
- **WHEN** a user views the sidebar
- **THEN** the "Channels" and "Playlists" titles appear in a heading style, larger and bolder than the rows they label

### Requirement: Sidebar Channel Ordering
The sidebar's Channels section SHALL list channels with an unwatched count above 0 first, ordered by unwatched count from highest to lowest, with equal counts ordered by name, ignoring case. The channels with no unwatched videos SHALL follow, ordered by name, ignoring case.

#### Scenario: Unread channels come first, most unread first
- **WHEN** the tracked channels are "Alpha" (0 unwatched), "Bravo" (2 unwatched), "Charlie" (7 unwatched) and "Delta" (0 unwatched)
- **THEN** the Channels section lists them in the order "Charlie", "Bravo", "Alpha", "Delta"

#### Scenario: Equal unwatched counts are ordered by name
- **WHEN** "zeta" and "Echo" both have 3 unwatched videos
- **THEN** "Echo" is listed before "zeta"

#### Scenario: Counts are compared as numbers
- **WHEN** "Alpha" has 9 unwatched videos and "Bravo" has 10
- **THEN** "Bravo" is listed before "Alpha"

#### Scenario: A channel's position follows its count
- **WHEN** a user watches enough of a channel's videos that its unwatched count drops to 0
- **THEN** the channel moves into the caught-up group at its alphabetical position

### Requirement: Sidebar Playlist Ordering
The sidebar's Playlists section SHALL list playlists by name, ignoring case.

#### Scenario: Playlists listed alphabetically
- **WHEN** the tracked playlists are "watch later", "Courses" and "Ambient"
- **THEN** the Playlists section lists them in the order "Ambient", "Courses", "watch later"

### Requirement: Collapsed Sidebar Sections
Each sidebar section SHALL start collapsed, showing only its leading rows in the section's order followed by a control that reveals the remaining rows and states how many there are. In the Channels section, the leading rows SHALL be the channels with unwatched videos, up to 10; when no channel has unwatched videos, the leading rows SHALL be the first 5 channels. In the Playlists section, the leading rows SHALL be the first 5 playlists. When a section has no rows beyond its leading rows, the control SHALL NOT be shown. An expanded section SHALL offer a control to collapse it again.

#### Scenario: Only unread channels shown while collapsed
- **WHEN** 3 of 40 tracked channels have unwatched videos and the Channels section is collapsed
- **THEN** the section shows those 3 channels followed by a control to show the 37 others

#### Scenario: Unread channels beyond the cap
- **WHEN** 14 tracked channels have unwatched videos and the Channels section is collapsed
- **THEN** the section shows the 10 with the most unwatched videos, and the other unread channels are among the hidden rows

#### Scenario: No unread channels
- **WHEN** no tracked channel has unwatched videos and the Channels section is collapsed
- **THEN** the section shows the first 5 channels alphabetically followed by a control to show the rest

#### Scenario: Short section has no control
- **WHEN** 4 playlists are tracked
- **THEN** the Playlists section shows all 4 and no show-more control

#### Scenario: Expanding and collapsing a section
- **WHEN** a user activates a section's show-more control and then its collapse control
- **THEN** the section first shows all its rows in order, then returns to its leading rows

### Requirement: Active Sidebar Entry Always Visible
When the user is on a channel's or playlist's detail view and that entry is not among its collapsed section's leading rows, the sidebar SHALL still show that entry, marked as active, in the collapsed section.

#### Scenario: Viewing a caught-up channel while collapsed
- **WHEN** a user opens the detail view of a channel that is hidden in the collapsed Channels section
- **THEN** the Channels section shows that channel, marked as active, alongside its leading rows

### Requirement: Remembered Sidebar Section State
The application SHALL remember, per browser, whether each sidebar section is expanded or collapsed, and SHALL restore that state when the application is loaded again. When no state has been remembered, a section SHALL start collapsed. When the remembered state cannot be read, the sidebar SHALL fall back to collapsed without an error.

#### Scenario: Expanded section survives a reload
- **WHEN** a user expands the Channels section and reloads the page
- **THEN** the Channels section is still expanded, and the Playlists section keeps its own state

### Requirement: Sidebar Search
When the number of tracked channels plus tracked playlists is above 15, the sidebar SHALL show a single search field above its sections. At 15 or fewer the field SHALL NOT be shown. While the field contains text, each section SHALL show every entry whose name contains that text, ignoring case, in the section's usual order and regardless of whether the section is collapsed. A section with no matching entry SHALL be hidden. When no entry matches in either section, the sidebar SHALL say that nothing matches. Clearing the field SHALL restore each section's normal collapsed or expanded view.

#### Scenario: Search field hidden for a short list
- **WHEN** 10 channels and 5 playlists are tracked
- **THEN** the sidebar shows no search field

#### Scenario: Search field shown for a long list
- **WHEN** 12 channels and 4 playlists are tracked
- **THEN** the sidebar shows a search field above the Channels section

#### Scenario: Matching across both sections
- **WHEN** a user types "ver" and the channel "Veritasium" and the playlist "Universe Overview" are tracked
- **THEN** the Channels section shows "Veritasium" and the Playlists section shows "Universe Overview", even if both sections are collapsed and those entries are not among their leading rows

#### Scenario: Section without matches is hidden
- **WHEN** a user's search text matches only channels
- **THEN** the Playlists section is not shown

#### Scenario: Clearing the search
- **WHEN** a user clears the search field
- **THEN** both sections return to the collapsed or expanded view they had before searching

### Requirement: Background Refresh Pauses In Hidden Tabs
The application SHALL NOT request fresh data from the daemon while its browser tab is hidden. When the tab becomes visible again, it SHALL refresh the data shown in the current view and sidebar.

#### Scenario: Hidden tab stops polling
- **WHEN** the application's tab is hidden
- **THEN** it makes no data requests until the tab is visible again

#### Scenario: Returning to the tab
- **WHEN** a user returns to the application's tab after new videos have downloaded
- **THEN** the sidebar badges and current view reflect the new videos without a manual reload

### Requirement: Sidebar Lists Refresh Periodically
While its tab is visible, the application SHALL refresh the sidebar's channel and playlist lists at least once every 60 seconds, so that changes made by the daemon or on another device appear without a page reload.

#### Scenario: Channel gains new videos in the background
- **WHEN** the daemon downloads a new video for a channel while the application is open and visible
- **THEN** that channel's badge and position in the sidebar update within 60 seconds without a page reload

### Requirement: Tasks View Describes Thumbnail Fetches
The tasks view SHALL describe a thumbnail fetch task in plain language,
naming the video whose thumbnail is being fetched and the playlist or
channel it belongs to, the same way it describes a video download task.

#### Scenario: Thumbnail fetch task with resolvable context
- **WHEN** the tasks view lists a thumbnail fetch task whose video title and playlist or channel name were resolved
- **THEN** the task is described as fetching the thumbnail of that video in that playlist or channel

#### Scenario: Thumbnail fetch task whose video can no longer be found
- **WHEN** the tasks view lists a thumbnail fetch task whose video title could not be resolved
- **THEN** the task is still described as a thumbnail fetch, using a generic placeholder in place of the missing names

### Requirement: Tasks View Search
The tasks view SHALL show a search field above the currently selected tab's list when that tab lists more than 15 tasks, and SHALL NOT show it otherwise. While the field contains text, the current tab SHALL list only the tasks whose plain-language description contains that text, ignoring case, keeping the tab's usual order. When no task in the current tab matches, the view SHALL say that nothing matches. Clearing the field SHALL restore the tab's full list. The search text SHALL apply only to the current tab; switching tabs SHALL clear it.

#### Scenario: Search field hidden for a short tab
- **WHEN** the selected tab lists 15 or fewer tasks
- **THEN** no search field is shown for it

#### Scenario: Search field shown for a long tab
- **WHEN** the selected tab lists more than 15 tasks
- **THEN** a search field is shown above its list

#### Scenario: Filtering by description
- **WHEN** a user types text that appears in some tasks' descriptions
- **THEN** the tab lists only the tasks whose description contains that text, ignoring case

#### Scenario: Nothing matches
- **WHEN** a user's search text matches no task in the current tab
- **THEN** the view says that nothing matches

#### Scenario: Clearing the search
- **WHEN** a user clears the search field
- **THEN** the tab returns to listing all of its tasks

### Requirement: Tasks View Describes Every Task Type
The tasks view SHALL describe every task it lists in plain language and SHALL NEVER display a task's raw type identifier. Each task family SHALL be described in human terms, including playlist, channel, and Plex collection syncs, video downloads and thumbnail fetches, file and container deletions, and the yt-dlp self-update. When a name the description would include could not be resolved, the description SHALL use a generic placeholder in its place rather than omitting the description.

#### Scenario: Plex collection sync is described in plain language
- **WHEN** the tasks view lists a Plex collection reconciliation task
- **THEN** it is described in plain language and not by its raw type identifier

#### Scenario: yt-dlp self-update is described in plain language
- **WHEN** the tasks view lists a yt-dlp self-update task
- **THEN** it is described in plain language and not by its raw type identifier

#### Scenario: No raw type identifier is ever shown
- **WHEN** the tasks view lists a task of any type
- **THEN** its row shows a human-readable description and never a raw snake_case type string

### Requirement: Tasks View Shows Last Error
When a task the tasks view lists has a recorded last error, the view SHALL show that error on the task's row, so a user can see why a task is being retried. A task with no recorded last error SHALL show none.

#### Scenario: Retrying task shows its last error
- **WHEN** the tasks view lists a task that has been retried and has a recorded last error
- **THEN** the task's row shows that error text

#### Scenario: Task without an error shows none
- **WHEN** the tasks view lists a task with no recorded last error
- **THEN** the task's row shows no error text

### Requirement: Tasks View Tabs
The tasks view SHALL group its tasks into tabs by task family, in this order: **Active**, **Downloads**, **Syncs**, and **Other**. Each tab SHALL show, beside its name, a count of the tasks it currently contains. A task belongs to a tab as follows:

- **Active**: every task whose status is `running`, regardless of its type.
- **Downloads**: video download and thumbnail fetch tasks.
- **Syncs**: playlist reconciliation and channel reconciliation tasks.
- **Other**: every task that belongs to neither Downloads nor Syncs, including Plex collection reconciliation, the yt-dlp self-update, deletion of a video file or of a playlist's or channel's files, and any task type the view does not otherwise recognise.

Every task SHALL appear in exactly one of Downloads, Syncs, or Other, and additionally in **Active** while it is running. The view SHALL open on the **Active** tab. Each tab SHALL list its tasks in the view's usual order (running first, then tasks already due, then tasks scheduled for later). Each task row SHALL show an icon that reflects the kind of work the task does (download, sync, deletion, or maintenance), so rows of different kinds within **Other** remain distinguishable.

#### Scenario: View opens on the Active tab
- **WHEN** a user opens the tasks view
- **THEN** the Active tab is selected and lists only the tasks whose status is `running`

#### Scenario: Running task appears under Active regardless of type
- **WHEN** a sync task is running
- **THEN** it appears in the Active tab as well as in the Syncs tab

#### Scenario: Downloads tab lists download and thumbnail tasks
- **WHEN** the tasks include video downloads and thumbnail fetches
- **THEN** the Downloads tab lists those tasks and no sync or other tasks

#### Scenario: Syncs tab lists only playlist and channel reconciles
- **WHEN** the tasks include playlist, channel, and Plex collection reconciliations
- **THEN** the Syncs tab lists the playlist and channel reconciliations and not the Plex collection reconciliation

#### Scenario: Other tab collects everything else
- **WHEN** the tasks include a Plex collection reconciliation, a yt-dlp self-update, a video-file deletion, a deleted container's file cleanup, and a task of an unrecognised type
- **THEN** the Other tab lists all of them and no download or playlist/channel sync tasks

#### Scenario: Counts match listed tasks
- **WHEN** tasks of several families are present
- **THEN** each tab's count matches the number of tasks it lists, and the Downloads, Syncs, and Other counts add up to the total number of tasks

#### Scenario: No Cleanup or All tab
- **WHEN** a user opens the tasks view
- **THEN** no Cleanup tab and no All tab are offered

#### Scenario: Active tab is empty
- **WHEN** no task is running
- **THEN** the Active tab shows a message that nothing is running rather than an empty area, and the other tabs still list their tasks

### Requirement: Tasks View Tab Bar Affordance
The tasks view's tab bar SHALL make each tab look clickable and SHALL make the selected tab unmistakable. The tasks view SHALL be limited to a readable width and centred in the space beside the sidebar, and the tab bar SHALL span exactly the width of the task list, its tabs sharing that width equally. The tab bar SHALL be drawn as a row of tabs over a horizontal rule, with the selected tab marked by an underline in the application's primary colour and its label in the full foreground colour and a heavier weight; unselected tabs SHALL show a muted label. Hovering an unselected tab SHALL visibly change its background and label colour. Each tab SHALL show an icon for its family, its name, and its count as a pill, with the selected tab's pill in the primary colour. On narrow screens each tab SHALL show only its icon and count, keeping its name as its accessible name, so that all tabs fit without horizontal scrolling. While at least one task is running, the **Active** tab SHALL show a live indicator beside its count. Each tab SHALL be at least 40 pixels tall and SHALL show a visible focus ring when focused from the keyboard. The tab bar SHALL look the same whatever the operating system's colour-scheme preference.

#### Scenario: Selected tab is marked
- **WHEN** a user views the tasks view with the Syncs tab selected
- **THEN** the Syncs tab shows the primary-coloured underline, a full-colour heavier label, and a primary-coloured count pill, and no other tab does

#### Scenario: Hovering an unselected tab
- **WHEN** a user hovers an unselected tab with a pointer
- **THEN** that tab's background and label colour change

#### Scenario: Live indicator on Active
- **WHEN** at least one task is running
- **THEN** the Active tab shows a live indicator beside its count

#### Scenario: No live indicator when idle
- **WHEN** no task is running
- **THEN** the Active tab shows no live indicator

#### Scenario: Keyboard focus
- **WHEN** a user moves keyboard focus onto a tab
- **THEN** that tab shows a visible focus ring

#### Scenario: Wide screen
- **WHEN** the tasks view is shown on a wide desktop screen
- **THEN** the view is centred at a readable width, and the tab bar's rule and the task list share the same left and right edges

#### Scenario: Narrow screen
- **WHEN** the tasks view is shown on a phone-width screen
- **THEN** every tab shows its icon and count without its visible name, all four tabs are visible without scrolling, and each tab is still announced by its name to assistive technology

### Requirement: Run Sync Now From Tasks View
Each playlist or channel reconciliation task listed in the **Syncs** tab SHALL offer a **Run now** action. Activating it SHALL trigger the on-demand reconcile of that task's playlist or channel — the same action the sidebar's sync offers — and SHALL leave the listed task itself and its scheduled run time unchanged. While the triggered reconcile is in progress, that row's action SHALL show that it is working and SHALL NOT accept another activation. When the reconcile completes, the tasks view SHALL refresh its list. When the reconcile fails, the view SHALL show the failure on that row without removing it. A reconciliation task whose status is `running` SHALL NOT offer Run now, since its sync is already underway. Tasks in other tabs SHALL NOT offer this action.

#### Scenario: Running a playlist sync now
- **WHEN** a user activates Run now on a playlist reconciliation task in the Syncs tab
- **THEN** the application triggers a reconcile of that playlist, and the task remains listed with its scheduled run time unchanged

#### Scenario: Running a channel sync now
- **WHEN** a user activates Run now on a channel reconciliation task in the Syncs tab
- **THEN** the application triggers a reconcile of that channel

#### Scenario: Run now while in progress
- **WHEN** a user has activated Run now on a row and the reconcile has not completed yet
- **THEN** that row's action indicates it is working and cannot be activated again

#### Scenario: Run now completes
- **WHEN** a triggered reconcile completes successfully
- **THEN** the tasks view refreshes its list of tasks

#### Scenario: Run now fails
- **WHEN** a triggered reconcile fails
- **THEN** the view shows that the sync failed, and the row's action can be activated again

#### Scenario: No Run now on a running sync
- **WHEN** the Syncs tab lists a reconciliation task whose status is `running`
- **THEN** that row offers no Run now action

#### Scenario: No Run now outside Syncs
- **WHEN** a user views the Downloads, Other, or Active tab
- **THEN** no task row offers a Run now action

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

### Requirement: Lowering The Video Limit Warns About Deletion
While the edit channel dialog's video limit is below the channel's current limit, the dialog SHALL show an always-visible warning that the next sync deletes the channel's downloaded videos beyond the newest ones within the new limit.

#### Scenario: Lowering the limit
- **WHEN** a user lowers a channel's video limit from 10 to 3 in the edit channel dialog
- **THEN** the dialog warns that downloaded videos beyond the 3 newest will be deleted

#### Scenario: Raising the limit
- **WHEN** a user raises the video limit or leaves it unchanged
- **THEN** no deletion warning is shown

### Requirement: Changing The Quality Notes It Applies To New Videos Only
While the edit channel dialog's selected video quality differs from the channel's current quality, the dialog SHALL show an informational (non-warning) note reading "The new quality applies to new videos only. Videos already downloaded keep their current quality."

#### Scenario: Changing the quality
- **WHEN** a user changes a channel's video quality from High to Low in the edit channel dialog
- **THEN** the dialog notes that the new quality applies to new videos only and that downloaded videos keep their current quality

#### Scenario: Quality unchanged
- **WHEN** the selected video quality equals the channel's current quality
- **THEN** no quality note is shown
