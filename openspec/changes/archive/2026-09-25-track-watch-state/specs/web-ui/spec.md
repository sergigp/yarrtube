## MODIFIED Requirements

### Requirement: Sidebar Row Actions
Each tracked channel or playlist row in the sidebar SHALL provide a visible control to trigger a sync (reconcile) and a visible control to delete it. Each tracked channel row SHALL also provide a visible control to mark the channel watched. Deleting SHALL require the user to confirm before it takes effect. Marking a channel watched SHALL take effect without confirmation. Deleting the channel or playlist whose detail view is currently open SHALL return the user to the home view.

#### Scenario: Syncing from the sidebar
- **WHEN** a user activates a sidebar row's sync control
- **THEN** the application triggers a reconcile for that channel or playlist

#### Scenario: Deleting from the sidebar requires confirmation
- **WHEN** a user activates a sidebar row's delete control
- **THEN** the application prompts for confirmation before deleting that channel or playlist

#### Scenario: Deleting the currently viewed entry
- **WHEN** a user confirms deletion of the channel or playlist whose detail view is currently open
- **THEN** the application navigates to the home view

#### Scenario: Marking a channel watched from the sidebar
- **WHEN** a user activates a channel row's mark-watched control
- **THEN** the application marks that channel watched, and its unwatched badge disappears

## ADDED Requirements

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
Each channel row in the sidebar SHALL display a badge with the channel's unwatched video count when that count is above 0, and no badge when it is 0. The badge SHALL update without a manual page reload.

#### Scenario: Channel with unwatched videos
- **WHEN** a channel has 3 downloaded, unwatched videos
- **THEN** its sidebar row displays a badge showing 3

#### Scenario: Channel with nothing unwatched
- **WHEN** a channel has no downloaded, unwatched videos
- **THEN** its sidebar row displays no badge

#### Scenario: Badge updates after watching
- **WHEN** a user finishes watching one of a channel's unwatched videos
- **THEN** the channel's badge count decreases without a page reload

### Requirement: Mark Channel Watched From Channel View
A channel detail view SHALL provide a visible control to mark the channel watched, which takes effect without confirmation.

#### Scenario: Marking a channel watched from its view
- **WHEN** a user activates the mark-watched control in a channel's detail view
- **THEN** every downloaded video in the list shows a tick, and the channel's sidebar badge disappears
