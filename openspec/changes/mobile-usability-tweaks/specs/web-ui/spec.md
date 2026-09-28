## MODIFIED Requirements

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

### Requirement: Home Page Layout
The home view SHALL display recently synced videos as a grid of cards, each showing the video's thumbnail with its title below it. For videos from a tracked channel, the card SHALL also show the channel's avatar and name.

#### Scenario: Home view with recent videos
- **WHEN** one or more videos have been synced
- **THEN** the home view renders them as a grid of thumbnail-and-title cards

#### Scenario: Channel video card
- **WHEN** a recent video comes from a tracked channel
- **THEN** its card shows the channel's avatar and name below the title

#### Scenario: Playlist video card
- **WHEN** a recent video comes from a tracked playlist
- **THEN** its card shows no channel avatar or name

## ADDED Requirements

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
In the video detail pane, the video's title SHALL occupy its own full-width row. No other elements (status or quality indicators) SHALL share that row at any viewport size.

#### Scenario: Long title on a mobile viewport
- **WHEN** a user on a mobile-width viewport selects a video with a long title
- **THEN** the title wraps across the full width of the detail pane, and the status and quality indicators appear below it

### Requirement: Collapsible Video Details
The video detail pane SHALL always show the video's title. The rest of the pane (status and quality indicators, file path, and the "Open on YouTube" link) SHALL sit in a section the user can expand and collapse. This section SHALL start collapsed on viewports narrower than the desktop breakpoint and expanded at or above it.

#### Scenario: Details start collapsed on mobile
- **WHEN** a user on a mobile-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the video's title and an expand control, and hides the indicators, path and YouTube link

#### Scenario: Expanding details
- **WHEN** a user activates the expand control in a collapsed detail pane
- **THEN** the status and quality indicators, file path and "Open on YouTube" link become visible, and the control collapses them again when activated

#### Scenario: Details start expanded on desktop
- **WHEN** a user on a desktop-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the title, indicators, path and YouTube link without needing to be expanded

### Requirement: Compact Mobile Player
On viewports narrower than the desktop breakpoint, the video player area SHALL be sized to the video's own aspect ratio, with no blank padding above or below the video. The gap between the header and the player SHALL be no more than the standard page gutter.

#### Scenario: Downloaded video on a mobile viewport
- **WHEN** a user on a mobile-width viewport views a downloaded 16:9 video in a detail view
- **THEN** the player area is exactly as tall as the video at full width, and it starts just below the header

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
