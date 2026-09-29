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
Each tracked channel or playlist row in the sidebar SHALL provide an always-visible menu control that does not depend on hover. The menu SHALL offer an action to trigger a sync (reconcile) and an action to delete the entry. For channel rows it SHALL also offer an action to mark the channel watched. The row's unwatched badge SHALL sit at the row's trailing edge, next to the menu control, with no reserved blank space between them. Deleting SHALL require the user to confirm before it takes effect. Marking a channel watched SHALL take effect without confirmation. Deleting the channel or playlist whose detail view is currently open SHALL return the user to the home view.

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

#### Scenario: Playlist rows have no mark-watched action
- **WHEN** a user opens a playlist row's menu
- **THEN** the menu offers sync and delete only

### Requirement: Collapsible Mobile Sidebar
On viewports narrower than the desktop breakpoint, the sidebar (channels and playlists navigation) SHALL be hidden by default and SHALL NOT occupy permanent layout space. The user SHALL be able to reveal it via a control in the header, and dismiss it via a close control, a backdrop tap, or selecting a navigation item within it. When open, it SHALL cover no more than about two thirds of a typical phone's width, leaving the view behind it visible. While it is open, the page behind it SHALL NOT scroll, whatever the sidebar contains. On viewports at or above the desktop breakpoint, the sidebar SHALL remain a persistent, always-visible panel, unaffected by this open/closed state.

#### Scenario: Opening the sidebar on a mobile viewport
- **WHEN** a user on a mobile-width viewport activates the header's sidebar control
- **THEN** the sidebar becomes visible over the current view

#### Scenario: Dismissing the sidebar on a mobile viewport
- **WHEN** a user on a mobile-width viewport, with the sidebar open, taps the close control, taps outside the sidebar, or selects a channel or playlist in it
- **THEN** the sidebar is hidden again

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

### Requirement: Add Dialog Storage Location
The add playlist dialog SHALL present the storage location in its main body, not inside "Advanced options". The add channel dialog SHALL present it inside its collapsed "Advanced options" section. In both, the location SHALL consist of two separate controls: a parent folder, and a folder name for the directory the videos are stored in.

The parent folder SHALL be set only through the parent folder browser; it SHALL NOT be free-text. Every directory in the resulting destination SHALL therefore be one the user either browsed into — and which consequently already exists — or named explicitly in the browser's create-folder step, with that location's existing subdirectories listed on screen at the time. No directory is ever brought into existence by unreviewed free-text entry.

The folder name SHALL be a single path segment. The dialog SHALL reject a folder name containing `/`, since allowing one would let unbrowsed intermediate directories back in through the field this control exists to replace. An empty folder name SHALL also be rejected.

The parent folder SHALL default to `playlists/` in the add playlist dialog and `channels/` in the add channel dialog. The folder name SHALL be pre-filled automatically rather than left blank: in the add playlist dialog, from a filesystem-safe slug of the entered playlist name; in the add channel dialog, from a filesystem-safe slug of the entered channel handle or URL. It SHALL keep recomputing as that source field changes. Once the user edits the folder name directly, the system SHALL stop overwriting it with further automatic updates for the remainder of that dialog session.

The playlist name and the folder name are distinct: the playlist name is the playlist's display name, and the folder name determines only the directory on disk.

#### Scenario: Location controls are visible without expanding advanced options
- **WHEN** a user opens the add playlist dialog
- **THEN** the parent folder control and the folder name field are visible without expanding "Advanced options"

#### Scenario: Channel location controls are inside advanced options
- **WHEN** a user opens the add channel dialog
- **THEN** the parent folder control and the folder name field are hidden inside the collapsed "Advanced options" section

#### Scenario: Parent folder defaults per mode
- **WHEN** a user opens an add dialog
- **THEN** the parent folder is `playlists/` in the add playlist dialog and `channels/` in the add channel dialog

#### Scenario: Folder name pre-fills from the playlist name
- **WHEN** a user types a playlist name in the add playlist dialog, without having edited the folder name field
- **THEN** the folder name's value updates to a filesystem-safe slug of that playlist name

#### Scenario: Folder name pre-fills from the channel handle or URL
- **WHEN** a user types a channel handle or URL in the add channel dialog, without having edited the folder name field
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
Both add dialogs SHALL let the user change the parent folder by browsing the directories under the videos root. The browser SHALL NOT be expanded by default; the dialog SHALL show the current parent folder and a control that reveals the browser. In the add channel dialog these sit inside "Advanced options". When revealed, the browser SHALL open on the current parent folder.

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

### Requirement: Add Dialog Destination Preview
The add playlist dialog SHALL display the absolute destination the videos will be downloaded to — the videos root, the parent folder, and the folder name joined together — and SHALL update it as the parent folder or folder name changes. Only this preview SHALL be labelled as the download destination; the parent folder control SHALL be labelled as the parent, so that neither can be read as the location videos are written to. The add channel dialog shows its destination through its destination notice instead.

The preview SHALL name every directory in the destination that does not exist yet and will be created, not only the final one. A parent adopted through the create-folder step does not exist either, so a destination can require more than one new directory; naming them all is what makes a mistyped parent visible before submission rather than after videos have downloaded into it.

The preview SHALL distinguish these cases:

- the destination exists already, and the videos will be added to its contents;
- one or more directories in the destination do not exist and will be created, each of them named;
- the destination is already the storage location of another playlist or channel.

In the last case the dialog SHALL report the conflict and SHALL NOT submit the request, so that a location already in use is refused before submission rather than after the server rejects it.

#### Scenario: Destination preview reflects the composed path
- **WHEN** a user changes either the parent folder or the folder name in the add playlist dialog
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

### Requirement: Add Channel Destination Notice
The add channel dialog SHALL show a notice between the channel handle or URL field and the "Advanced options" section once that field is not empty. The notice SHALL state how many of the channel's latest videos will be downloaded (the video limit) and the absolute destination they will be downloaded to (the videos root, parent folder and folder name joined together). It SHALL update as the handle, video limit, parent folder or folder name changes. The notice SHALL offer a "change" action that expands "Advanced options".

When the destination is already the storage location of another playlist or channel, the notice SHALL instead be shown as an error that names the destination and the playlist or channel occupying it, still offering the "change" action, and the dialog SHALL NOT submit the request.

The add channel dialog SHALL NOT show a separate destination preview or report which directories will be created or already exist.

#### Scenario: Notice hidden before a handle is entered
- **WHEN** a user opens the add channel dialog and the channel handle or URL field is empty
- **THEN** no destination notice is shown

#### Scenario: Notice states the video limit and destination
- **WHEN** a user enters `@veritasium` in the add channel dialog without changing any advanced option
- **THEN** the notice states that the latest 3 videos from the channel will be downloaded to the videos root joined with `channels/veritasium`

#### Scenario: Notice follows the advanced options
- **WHEN** a user changes the video limit, the parent folder or the folder name
- **THEN** the notice's video count and destination update to match

#### Scenario: Change action expands advanced options
- **WHEN** a user activates the notice's "change" action
- **THEN** "Advanced options" expands, showing the storage location controls

#### Scenario: Destination already in use
- **WHEN** the composed destination is the storage location of another playlist or channel
- **THEN** the notice is shown as an error naming the destination and the playlist or channel occupying it, and the dialog does not submit the request

### Requirement: Add Dialog Download Options
Both add dialogs SHALL present video quality inside a collapsed "Advanced options" section that is not expanded by default; the add channel dialog SHALL also present the video limit and the storage location there. The video quality control SHALL be labeled "Video quality" and SHALL offer a tooltip explaining that it controls the download resolution and that a lower resolution reduces storage use. In the add channel dialog, the video limit field SHALL default to 3 and SHALL accept whole numbers from 1 to 1000.

In the add playlist dialog the storage location is not part of this section; it is presented in the dialog's main body.

#### Scenario: Advanced options start collapsed
- **WHEN** a user opens either add dialog
- **THEN** the video quality control and, in the add channel dialog, the video limit field and storage location are hidden inside a collapsed "Advanced options" section

#### Scenario: Video quality tooltip
- **WHEN** a user reveals the "Video quality" tooltip
- **THEN** it explains that the setting controls the download resolution and that choosing a lower resolution saves storage

#### Scenario: Channel video limit default
- **WHEN** a user opens the add channel dialog and does not change the video limit
- **THEN** the video limit field defaults to 3

#### Scenario: Channel video limit range
- **WHEN** a user enters a video limit below 1 or above 1000 in the add channel dialog
- **THEN** the dialog flags the field as invalid and does not submit the request

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
While a video plays in a playlist or channel detail view, the application SHALL report the current playback position to the daemon at most every 15 seconds. It SHALL also report the position when playback pauses or ends, when the user switches to another video or leaves the view, and when the page is closed or hidden.

#### Scenario: Progress reported while playing
- **WHEN** a video has been playing for 15 seconds or more since the last report
- **THEN** the application reports the current position

#### Scenario: Progress reported on pause
- **WHEN** a user pauses a playing video
- **THEN** the application reports the current position

#### Scenario: Progress reported on leaving
- **WHEN** a user switches video, navigates away, or closes the tab while a video is playing
- **THEN** the application reports the last position

### Requirement: Watched Tick On Video Thumbnails
Every video thumbnail in the playlist detail, channel detail and home views SHALL display a tick when that video is watched, and SHALL NOT display one when it is unwatched.

#### Scenario: Watched video thumbnail
- **WHEN** a watched video is listed in a playlist, channel or home view
- **THEN** its thumbnail displays a tick

#### Scenario: Unwatched video thumbnail
- **WHEN** an unwatched video is listed
- **THEN** its thumbnail does not display a tick

### Requirement: Unwatched Badge On Sidebar Channels
Each channel row in the sidebar SHALL display a badge with the channel's unwatched video count when that count is above 0, and no badge when it is 0. The badge SHALL update without a manual page reload. After an action in the application that changes a channel's unwatched count (a video becoming watched or unwatched during playback, marking the channel watched, syncing, adding or deleting a channel), the badge and the channel's sidebar position SHALL update promptly, without waiting for the periodic refresh. During playback, the channel list SHALL be refetched only when a progress report changes the video's watched state.

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

### Requirement: Mark Channel Watched From Channel View
A channel detail view SHALL provide a visible control to mark the channel watched, which takes effect without confirmation. The control SHALL sit in the detail view's page header.

#### Scenario: Marking a channel watched from its view
- **WHEN** a user activates the mark-watched control in a channel's detail view
- **THEN** every downloaded video in the list shows a tick, and the channel's sidebar badge disappears

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
A playlist or channel detail view SHALL open with a page header showing the playlist's or channel's name, with the channel's avatar beside it for channels. Below the name it SHALL show a summary of the number of videos and, when above 0, the number of unwatched videos. The header SHALL provide actions to sync (reconcile) and to delete the entry. For channels, it SHALL also provide an action to mark the channel watched. Deleting SHALL require confirmation and, once confirmed, SHALL return the user to the home view. On viewports narrower than the small breakpoint, the actions SHALL show as icons only, each keeping an accessible label.

#### Scenario: Channel page header
- **WHEN** a user opens a channel's detail view
- **THEN** the page header shows the channel's avatar, its name, a video count summary, and sync, mark-watched and delete actions

#### Scenario: Playlist page header
- **WHEN** a user opens a playlist's detail view
- **THEN** the page header shows the playlist's name, a video count summary, and sync and delete actions, with no avatar and no mark-watched action

#### Scenario: Summary omits unwatched count when nothing is unwatched
- **WHEN** a detail view's channel or playlist has no unwatched videos
- **THEN** the summary shows only the video count

#### Scenario: Syncing from the page header
- **WHEN** a user activates the sync action in a detail view's page header
- **THEN** the application triggers a reconcile for that channel or playlist and shows it as in progress until it completes

#### Scenario: Deleting from the page header
- **WHEN** a user activates the delete action in a detail view's page header and confirms
- **THEN** the application deletes that channel or playlist and navigates to the home view

#### Scenario: Page header actions on a narrow viewport
- **WHEN** a user views a detail view on a viewport narrower than the small breakpoint
- **THEN** the page header's actions show as icons only, and the name remains visible beside them

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
