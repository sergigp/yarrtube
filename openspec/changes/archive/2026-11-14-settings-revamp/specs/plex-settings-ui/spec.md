# Spec Delta

## Purpose

The Plex section of the web UI's Settings page: the integration's connection and sync status, a way to run each Plex sync now, and how category playlists behave in Plex.

## ADDED Requirements

### Requirement: Plex Section
The Settings page SHALL have a "Plex" section at `/settings/plex` with three groups, in order: "Connection", "Syncs" and "Category playlists". Its status SHALL come from `GET /api/plex` and refresh while the section is open.

#### Scenario: Opening the section
- **WHEN** a user opens `/settings/plex` with Plex configured
- **THEN** the page shows the "Connection", "Syncs" and "Category playlists" groups

### Requirement: Plex Not Configured
When Plex isn't configured, the Plex section SHALL say so, name `YARRTUBE_PLEX_URL`, `YARRTUBE_PLEX_TOKEN` and `YARRTUBE_PLEX_VIDEOS_PATH` as the variables to set, link to the Plex guide, and show neither the syncs nor the playlist options.

#### Scenario: Integration off
- **WHEN** a user opens `/settings/plex` and Plex isn't configured
- **THEN** the section names the three variables, links to the Plex guide, and shows no "Sync now" button

### Requirement: Plex Connection Status
The "Connection" group SHALL show "Connected" or "Can't reach Plex" with the error, the Plex Media Server version without its build suffix, the libraries holding yarrtube's folders, and when this was seen, relative to now. Before any sync has finished it SHALL say the status shows after the first sync.

#### Scenario: Connected
- **WHEN** the latest sync reached Plex `1.43.5.11079-b3b13e506` 2 minutes ago with the libraries "Subscriptions" and "Sergi's Playlists"
- **THEN** the group shows "Connected", "Plex Media Server 1.43.5.11079", both libraries and "2 minutes ago"

#### Scenario: Unreachable
- **WHEN** the latest sync failed with "connection refused"
- **THEN** the group shows "Can't reach Plex" and "connection refused", and the version and libraries last seen

#### Scenario: No sync yet
- **WHEN** Plex is configured and no sync has finished
- **THEN** the group says the status shows after the first sync

### Requirement: Plex Sync Rows
The "Syncs" group SHALL show a "Watch state" row and a "Labels & playlists" row, each with its last run (relative time, and its error when it failed), its next run, and a "Sync now" button. While that sync runs or a request for it is in flight, the button SHALL be disabled and read "Syncing…".

#### Scenario: Idle sync
- **WHEN** the watch sync last ran 3 minutes ago without error and is next due in 2 minutes
- **THEN** its row shows "3 minutes ago", "in 2 minutes" and an enabled "Sync now"

#### Scenario: Failed run
- **WHEN** the label sync's last run failed with "timed out"
- **THEN** its row shows that run's time and "timed out"

#### Scenario: Syncing now
- **WHEN** a user activates "Sync now" on "Watch state"
- **THEN** `POST /api/plex/syncs/watch_state` is sent, the button reads "Syncing…" disabled until the sync finishes, and the row then shows the new last run

#### Scenario: Request fails
- **WHEN** a user activates "Sync now" and the request fails
- **THEN** the row shows the error and the button is enabled again

### Requirement: Category Playlist Options
The "Category playlists" group SHALL offer "Watched episodes", either "Stay in the playlist" or "Drop out", and "Order", either "Newest first" or "Oldest first", filled with the saved preferences. Choosing one SHALL save it at once with only that preference and show "Saved". The group SHALL say Plex Web and the Mac app show no watched badge in playlists.

#### Scenario: Defaults
- **WHEN** a user opens the section and nothing has been saved
- **THEN** "Stay in the playlist" and "Newest first" are selected

#### Scenario: Dropping watched episodes
- **WHEN** a user chooses "Drop out"
- **THEN** a `PATCH /api/preferences` with only `plex_playlist_watched` `"drop"` is sent and "Saved" is shown

#### Scenario: Save fails
- **WHEN** a user chooses "Oldest first" and the save fails
- **THEN** the error is shown and "Newest first" is selected again

### Requirement: Playlist Options Need Categories
While categories are off, the "Category playlists" options SHALL be disabled, with a note that category playlists need categories on.

#### Scenario: Categories off
- **WHEN** `categories_enabled` is off and a user opens `/settings/plex`
- **THEN** both playlist options are disabled and the note is shown
