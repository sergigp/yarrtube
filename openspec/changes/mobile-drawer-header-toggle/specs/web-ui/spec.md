## MODIFIED Requirements

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
