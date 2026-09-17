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

### Requirement: Unified Add Dialog
The application SHALL provide a single entry point — one "Add" control, always available — that opens one dialog for adding either a playlist or a channel, with a switcher inside the dialog to choose which.

#### Scenario: Opening the add dialog
- **WHEN** a user clicks the "Add" control
- **THEN** the application opens a single dialog offering a choice between adding a playlist and adding a channel

#### Scenario: Switching between playlist and channel mode
- **WHEN** a user changes the dialog's switcher from one mode to the other
- **THEN** the dialog's fields change to match the selected mode

### Requirement: Add Dialog Advanced Options
The add dialog SHALL present a storage path field, video quality, and, in channel mode, the video limit, inside a collapsed "Advanced options" section that is not expanded by default. The video quality control SHALL be labeled "Video quality" and SHALL offer a tooltip explaining that it controls the download resolution and that a lower resolution reduces storage use. In channel mode, the video limit field SHALL default to 3.

The path field SHALL be pre-filled automatically rather than left blank: in playlist mode, from a filesystem-safe slug of the entered name, prefixed `playlists/`; in channel mode, from a filesystem-safe slug of the entered channel handle or URL, prefixed `channels/`. It SHALL keep recomputing as the source field (name or channel handle/URL) changes, and SHALL remain a plain editable field at all times. Once the user edits the path field directly, the system SHALL stop overwriting it with further automatic updates for the remainder of that dialog session.

#### Scenario: Advanced options start collapsed
- **WHEN** a user opens the add dialog in either mode
- **THEN** the path field, video quality control, and (in channel mode) the video limit field are hidden inside a collapsed "Advanced options" section

#### Scenario: Path pre-fills from the playlist name
- **WHEN** a user is in playlist mode and types a name, without having edited the path field
- **THEN** the path field's value updates to a filesystem-safe slug of that name, prefixed `playlists/`

#### Scenario: Path pre-fills from the channel handle or URL
- **WHEN** a user is in channel mode and types a channel handle or URL, without having edited the path field
- **THEN** the path field's value updates to a filesystem-safe slug of that handle, prefixed `channels/`

#### Scenario: Manual path edit stops further auto-fill
- **WHEN** a user types directly into the path field, and then continues editing the name or channel handle/URL field
- **THEN** the path field's value no longer changes in response to those edits

#### Scenario: Video quality tooltip
- **WHEN** a user reveals the "Video quality" tooltip
- **THEN** it explains that the setting controls the download resolution and that choosing a lower resolution saves storage

#### Scenario: Channel video limit default
- **WHEN** a user opens the add dialog in channel mode and does not change the video limit
- **THEN** the video limit field defaults to 3

### Requirement: Advanced Options Auto-Expand On Path Conflict
If a create-playlist or create-channel request fails because the submitted path is already in use, the add dialog SHALL expand "Advanced options" automatically, so the path field and the error message are visible without further action.

#### Scenario: Submission fails due to a path already in use
- **WHEN** a user submits the add dialog and the server rejects the request because the path is already used by another playlist or channel
- **THEN** the dialog expands "Advanced options" (if collapsed) and displays the error, leaving the path field visible and editable

### Requirement: Add Dialog Dismissal
The add dialog SHALL NOT close in response to a click outside the dialog. It SHALL close in response to the Escape key or its explicit close control.

#### Scenario: Clicking outside the dialog
- **WHEN** the add dialog is open and a user clicks outside it
- **THEN** the dialog remains open

#### Scenario: Closing via Escape or the close control
- **WHEN** the add dialog is open and a user presses Escape or activates its close control
- **THEN** the dialog closes

### Requirement: Detail View Fixed Video Area
In a playlist or channel detail view, the video player and the selected video's detail pane SHALL remain fixed in place as the user scrolls; only the video list SHALL scroll.

#### Scenario: Scrolling the video list
- **WHEN** a user scrolls the video list in a playlist or channel detail view
- **THEN** the video player and the video detail pane do not move

### Requirement: Video Detail Links To YouTube
A selected video's detail pane SHALL include a link that opens that video on YouTube in a new tab.

#### Scenario: Opening a video on YouTube
- **WHEN** a user activates the "Open on YouTube" link for a selected video
- **THEN** a new tab opens to that video's page on youtube.com
