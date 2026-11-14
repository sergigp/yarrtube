# Spec Delta

## MODIFIED Requirements

### Requirement: Mobile Navigation Coverage
The suite SHALL verify on the iPhone browser that:
- tapping a source in the open drawer navigates to it and closes the drawer
- Escape closes the drawer
- dismissing the drawer without navigating restores the previous scroll position
- the page behind the open drawer does not scroll
- the Tasks view's filter chips keep their visible names and a sync's "Sync now" or "Run now" works
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
- **WHEN** a user on a phone opens the Tasks view, taps the **Syncs** chip by its visible name, then a sync's "Run now"
- **THEN** the Syncs chip is selected and the sync runs without error

#### Scenario: Sync from the page header menu
- **WHEN** a user on a phone opens a playlist's view
- **THEN** no sync button is shown outside the "⋮" menu, and tapping the menu's "Sync" item triggers a reconcile that succeeds

### Requirement: Source Settings And Task Coverage
The suite SHALL verify that the Tasks view lists a download under **Running now** while it is running, and that channel avatars load.

#### Scenario: Download listed while running
- **WHEN** a newly added playlist's video is downloading
- **THEN** the Tasks view lists its download under **Running now**

#### Scenario: Channel avatar loads
- **WHEN** a tracked channel's avatar is shown in the sidebar
- **THEN** the image has loaded with non-zero natural dimensions

### Requirement: Errored Download Coverage
The suite SHALL verify that a video whose download fails is shown as Retrying with the reason the downloader reported and its next attempt, that the Tasks view shows its download under **Needs attention** with the reason, and that "Retry now" runs it again at once, after which the next attempt is further away, and that "Stop retrying" on the Tasks view's card makes the download Failed.

#### Scenario: Download fails
- **WHEN** a user adds the fixture playlist holding a video whose download always fails
- **THEN** the video is shown as Retrying with the failure reason and "next attempt in 10 minutes", the Tasks view shows its download under **Needs attention** with the reason, and after "Retry now" and another failure the video shows "next attempt in 1 hour"

#### Scenario: Stop retrying from the Tasks view
- **WHEN** a user activates "Stop retrying" on that download's card in the Tasks view
- **THEN** the card is gone and the video is shown as Failed

### Requirement: Search Coverage
The suite SHALL verify the sidebar's search, which only appears once there are more than 15 sources, and the Tasks view's search, including that the sidebar's search sits above the Categories section and filters categories too.

#### Scenario: Sidebar search
- **WHEN** more than 15 playlists are tracked and a user types part of one's name into the sidebar search
- **THEN** the sidebar lists only the matching playlists

#### Scenario: Sidebar search filters categories
- **WHEN** more than 15 playlists are tracked, the category "Science" exists, and a user types "scien" and then a playlist's name into the sidebar search
- **THEN** the search field is above the Categories section, the first search lists only "Science" with no Playlists section, and the second lists only that playlist with no Categories section

#### Scenario: Tasks search
- **WHEN** a user types part of a playlist's name into the Tasks view's search
- **THEN** only that playlist's tasks are listed
