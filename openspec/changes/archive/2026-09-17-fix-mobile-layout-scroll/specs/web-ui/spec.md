## MODIFIED Requirements

### Requirement: Detail View Fixed Video Area
In a playlist or channel detail view, the video player and the selected video's detail pane SHALL remain fixed in place as the user scrolls; only the video list SHALL scroll. This SHALL hold at all viewport sizes, including mobile.

#### Scenario: Scrolling the video list
- **WHEN** a user scrolls the video list in a playlist or channel detail view
- **THEN** the video player and the video detail pane do not move

#### Scenario: Scrolling the video list on a mobile viewport
- **WHEN** a user on a mobile-width viewport scrolls the video list in a playlist or channel detail view
- **THEN** the video player and the video detail pane do not move

## ADDED Requirements

### Requirement: Persistent Header
The application header SHALL remain fixed in place (visible, non-scrolling) as the user scrolls any view, at all viewport sizes, including mobile.

#### Scenario: Scrolling a view with a tall content area
- **WHEN** a user scrolls a view whose content exceeds the visible viewport height
- **THEN** the header remains visible in place and does not move with the scrolled content

#### Scenario: Scrolling on a mobile viewport as the browser's UI chrome shows or hides
- **WHEN** a user on a mobile browser scrolls a view while the browser's own toolbar collapses or expands
- **THEN** the header remains visible in place and does not move with the scrolled content

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
