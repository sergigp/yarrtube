## ADDED Requirements

### Requirement: Mobile Browser Projects
The suite SHALL run every spec tagged as mobile in an emulated iPhone browser running WebKit, with touch input and a mobile viewport, and only there. Specs SHALL NOT emulate a phone by resizing the desktop browser. Untagged specs SHALL run in desktop Chromium only.

#### Scenario: A mobile-tagged spec runs on the iPhone
- **WHEN** the full suite runs
- **THEN** each mobile-tagged spec is reported only for the iPhone/WebKit browser

#### Scenario: A desktop spec stays on desktop
- **WHEN** the full suite runs
- **THEN** an untagged spec is reported only for desktop Chromium

### Requirement: Mobile Watch Flow Coverage
The suite SHALL verify on the iPhone browser that tapping a video card on the home page opens that video's source with that video selected, that it plays, and that after recording a position and reloading, playback resumes from it.

#### Scenario: Tap a home card and watch
- **WHEN** a downloaded video is on the home page and the user taps its card on a phone
- **THEN** the source's view opens with that video's title in the detail pane, and the player plays it

#### Scenario: Resume after reload on a phone
- **WHEN** the user pauses halfway through and reloads the page
- **THEN** the player resumes from a position after the start

### Requirement: Mobile Layout Coverage
The suite SHALL verify on the iPhone browser that:
- no view (home, playlist, channel, category, tasks) scrolls horizontally
- the player stays directly below the header while the video list scrolls
- the video details start collapsed and "More" expands them
- the add, edit-settings and delete-confirm dialogs fit inside the viewport with their submit button reachable

#### Scenario: No horizontal overflow
- **WHEN** a user opens any view on a phone
- **THEN** the page is no wider than the viewport

#### Scenario: Sticky player
- **WHEN** a user scrolls a playlist's video list on a phone
- **THEN** the player's top edge stays at the header's bottom edge

#### Scenario: Collapsed video details
- **WHEN** a user opens a video on a phone
- **THEN** its description and "Open on YouTube" link are hidden until the user taps "More"

#### Scenario: Dialogs fit the phone screen
- **WHEN** a user opens the Add playlist dialog (with the folder browser), the Edit settings dialog or the Delete confirmation on a phone
- **THEN** the dialog lies within the viewport and its submit button can be tapped to complete the action

### Requirement: Mobile Navigation Coverage
The suite SHALL verify on the iPhone browser that:
- tapping a source in the open drawer navigates to it and closes the drawer
- Escape closes the drawer
- dismissing the drawer without navigating restores the previous scroll position
- the page behind the open drawer does not scroll
- icon-only controls (Tasks tabs, "Run now", the header's Sync) keep accessible names and work

#### Scenario: Navigate from the drawer
- **WHEN** a user opens the drawer on a phone and taps a tracked playlist
- **THEN** the playlist's view is shown and the drawer is closed

#### Scenario: Escape closes the drawer
- **WHEN** the drawer is open and the user presses Escape
- **THEN** the drawer is closed

#### Scenario: Background stays still
- **WHEN** the drawer is open and the user scrolls over the page behind it
- **THEN** the page's scroll position does not change, and it is unchanged after the drawer closes

#### Scenario: Icon-only controls
- **WHEN** a user on a phone opens the Tasks view and taps the Syncs tab by its accessible name, then a sync's "Run now"
- **THEN** the Syncs tab is selected and the sync runs without error

### Requirement: Media Compatibility Coverage
The suite SHALL verify that every downloaded video lands as an `.mp4` file with H.264 video and AAC audio, that the media endpoint answers a byte-range request with a partial response, and that the video plays in the iPhone/WebKit browser.

#### Scenario: Codec check
- **WHEN** a playlist's video has downloaded
- **THEN** the file is `.mp4` and its video stream is `h264` and its audio stream `aac`

#### Scenario: Range request
- **WHEN** the suite requests the video's media URL with `Range: bytes=0-1023`
- **THEN** the response is `206` with a `Content-Range` header and 1024 bytes

### Requirement: Category Detail Coverage
The suite SHALL verify that a category's view lists the downloaded videos of its sources as cards and that a card opens its video, selected, in its source's view.

#### Scenario: Play from a category
- **WHEN** a category holds a channel with a downloaded video and the user opens the category and taps that video's card
- **THEN** the channel's view opens with that video's title in the detail pane, and it plays

### Requirement: Playback Speed Coverage
The suite SHALL verify that choosing a speed from the video detail's speed menu changes the player's playback rate and that the menu shows the chosen speed.

#### Scenario: Change speed
- **WHEN** a user chooses 1.5x from the speed menu of a downloaded video
- **THEN** the player's playback rate is 1.5 and the menu shows 1.5x

### Requirement: Deep Link Coverage
The suite SHALL verify that reloading the page on each client route (home, playlist, channel, category, tasks) shows that view, and that opening the route of a source that is not tracked shows an error state rather than a blank page.

#### Scenario: Reload a deep link
- **WHEN** a user reloads the page on `/channels/<handle>`, `/categories/<id>` or `/tasks`
- **THEN** that view is shown

#### Scenario: Unknown source
- **WHEN** a user opens `/playlists/<id>` for a playlist that is not tracked
- **THEN** the view says "Playlist not found."

### Requirement: Source Settings And Task Coverage
The suite SHALL verify that the Tasks view's Downloads tab lists a download while it is running, and that channel avatars load.

#### Scenario: Download listed while running
- **WHEN** a newly added playlist's video is downloading
- **THEN** the Downloads tab lists its download

#### Scenario: Channel avatar loads
- **WHEN** a tracked channel's avatar is shown in the sidebar
- **THEN** the image has loaded with non-zero natural dimensions

### Requirement: Delete During Download Coverage
The suite SHALL verify that deleting a source whose downloads have not finished leaves nothing of it on disk.

#### Scenario: Delete mid-download
- **WHEN** a user deletes a playlist whose video is still downloading
- **THEN** the playlist's folder is removed and does not reappear once the in-flight work ends

### Requirement: Suite Health Checks
Every test SHALL fail on:
- an uncaught page error
- a `console.error` message not on an explicit allowlist
- a failed request for media or avatars
- a 5xx `/api` response
- a 4xx `/api` response the test did not declare as expected

Image assertions SHALL require the image to have loaded, not merely be present. A full passing run SHALL fail if any router `/api` route was never answered with a 2xx status.

#### Scenario: Unexpected console error
- **WHEN** the page logs a `console.error` that is not allowlisted during a test
- **THEN** the test fails naming the message

#### Scenario: Route only hit with errors
- **WHEN** a full passing run hit a route but every response to it was 4xx
- **THEN** the run fails listing that route

#### Scenario: Broken thumbnail
- **WHEN** a video card shows its placeholder because the thumbnail failed to load
- **THEN** the thumbnail assertion fails

## MODIFIED Requirements

### Requirement: Mobile Sidebar Coverage
The system SHALL verify, in the iPhone browser, that the sidebar can be opened via the header's menu control and closed again by that control or by tapping the backdrop, with the control's expanded state reflecting whether the sidebar is open.

#### Scenario: Open and close the mobile sidebar
- **WHEN** a user on a phone taps the menu button
- **THEN** the sidebar becomes visible and the control reports it as expanded, and tapping the control or the backdrop hides the sidebar and the control reports it as collapsed

### Requirement: Add-Dialog Error Handling Coverage
The system SHALL verify that the Add dialog surfaces a visible error message when creating a playlist with a path already used by another playlist, when creating a channel with an invalid handle, and when entering a playlist id YouTube does not know.

#### Scenario: Duplicate playlist path
- **WHEN** a user submits the Add dialog for a playlist whose path is already used by another tracked playlist
- **THEN** the dialog shows an error message and the Advanced options section is expanded

#### Scenario: Invalid channel handle
- **WHEN** a user submits the Add dialog for a channel with a handle that does not resolve to a real channel
- **THEN** the dialog shows an error message

#### Scenario: Unknown playlist
- **WHEN** a user enters a well-formed playlist id that YouTube does not know
- **THEN** the notice says the playlist does not exist and Create Playlist stays disabled
