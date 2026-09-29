## ADDED Requirements

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

## MODIFIED Requirements

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
