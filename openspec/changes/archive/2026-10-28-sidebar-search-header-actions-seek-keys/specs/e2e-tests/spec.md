# Spec Delta

## MODIFIED Requirements

### Requirement: Search Coverage
The suite SHALL verify the sidebar's search and the Tasks view's search, which only appear once there are more than 15 sources or tasks, including that the sidebar's search sits above the Categories section and filters categories too.

#### Scenario: Sidebar search
- **WHEN** more than 15 playlists are tracked and a user types part of one's name into the sidebar search
- **THEN** the sidebar lists only the matching playlists

#### Scenario: Sidebar search filters categories
- **WHEN** more than 15 playlists are tracked, the category "Science" exists, and a user types "scien" and then a playlist's name into the sidebar search
- **THEN** the search field is above the Categories section, the first search lists only "Science" with no Playlists section, and the second lists only that playlist with no Categories section

#### Scenario: Tasks search
- **WHEN** a user types part of a playlist's name into the Tasks view's search
- **THEN** only that playlist's tasks are listed

### Requirement: Mobile Navigation Coverage
The suite SHALL verify on the iPhone browser that:
- tapping a source in the open drawer navigates to it and closes the drawer
- Escape closes the drawer
- dismissing the drawer without navigating restores the previous scroll position
- the page behind the open drawer does not scroll
- icon-only controls (Tasks tabs, "Run now") keep accessible names and work
- a detail view's page header offers sync only through its "⋮" menu, and syncing from there works

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

#### Scenario: Sync from the page header menu
- **WHEN** a user on a phone opens a playlist's view
- **THEN** no sync button is shown outside the "⋮" menu, and tapping the menu's "Sync" item triggers a reconcile that succeeds

## ADDED Requirements

### Requirement: Page Header Actions Coverage
The suite SHALL verify on the desktop browser that a detail view's page header shows every action as a button named after it, with no "⋮" control.

#### Scenario: Desktop page header
- **WHEN** a user opens a playlist's view on a desktop viewport
- **THEN** buttons named "Sync", "Mark all watched", "Exclude from home", "Hide unwatched count", "Edit settings" and "Delete" are visible, and no "More actions for" control exists

### Requirement: Keyboard Seeking Coverage
The suite SHALL verify that ← and → seek a downloaded video 10 seconds with on-screen feedback, and that arrows typed into a text field do not seek.

#### Scenario: Seeking with the arrow keys
- **WHEN** a downloaded video is paused 30 seconds in and the user presses → twice, waits, then presses ←
- **THEN** the feedback reads "10 seconds" then "20 seconds" with the video 50 seconds in, it disappears on its own, and after ← the video is 40 seconds in

#### Scenario: Arrows in a text field
- **WHEN** the focus is in a text field and the user presses ←
- **THEN** the video does not seek and no feedback shows
