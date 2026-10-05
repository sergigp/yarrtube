## ADDED Requirements

### Requirement: Video Actions Menu
Every video card on the home view, every row of a playlist or channel detail view's video list except the selected video's row, and the title row of the video detail pane SHALL show an always-visible vertical "⋮" control to the right of the video's title. Activating it SHALL open a menu with a "Mark as watched" item, without selecting or navigating to the video. Choosing the item SHALL mark the video watched without confirmation. The item SHALL be disabled when the video is already watched or has not finished downloading. When marking fails, the application SHALL tell the user and leave the video as it was. The selected video's row SHALL keep the space the control would take, so rows stay aligned when the selection changes.

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

## MODIFIED Requirements

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
