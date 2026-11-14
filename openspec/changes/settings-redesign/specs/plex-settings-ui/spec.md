## MODIFIED Requirements

### Requirement: Plex Section
The Settings page SHALL have a "Plex" section at `/settings/plex`, titled "Plex". Under the title SHALL be "Keep watched status in sync with Plex and turn categories into Plex playlists." and a "Plex guide" link. Below them SHALL be three cards, in order: "Server", "Sync" and "Category playlists". Its status SHALL come from `GET /api/plex` and refresh while the section is open. After a playlist option is saved, "Saved" SHALL show next to the title for about 2 seconds.

#### Scenario: Opening the section
- **WHEN** a user opens `/settings/plex` with Plex configured
- **THEN** the page shows the "Server", "Sync" and "Category playlists" cards

### Requirement: Plex Not Configured
When Plex isn't configured, the Plex section SHALL show only a setup card. The card SHALL have:
- the heading "Watch your downloads in Plex";
- the text "Connect yarrtube to your Plex server and every channel and playlist shows up as a TV show.";
- three benefits: "New episodes appear in Plex right after they download.", "Watched status stays in sync between yarrtube and Plex." and "Each category becomes a smart playlist.";
- "Set these environment variables and restart the container", followed by one row each for `YARRTUBE_PLEX_URL` ("your server's address"), `YARRTUBE_PLEX_TOKEN` ("your Plex token") and `YARRTUBE_PLEX_VIDEOS_PATH` ("where Plex sees the videos folder");
- an "Open the Plex guide" link.

Each variable row SHALL have a "Copy" button (accessible name "Copy <variable>") that copies the variable's name to the clipboard and reads "Copied" for about 2 seconds. The section SHALL show neither the syncs nor the playlist options.

#### Scenario: Integration off
- **WHEN** a user opens `/settings/plex` and Plex isn't configured
- **THEN** the section shows "Watch your downloads in Plex", the three variables and "Open the Plex guide", and no "Sync now" button

#### Scenario: Copying a variable
- **WHEN** a user activates "Copy YARRTUBE_PLEX_TOKEN"
- **THEN** the clipboard holds `YARRTUBE_PLEX_TOKEN` and the button reads "Copied"

### Requirement: Plex Connection Status
The "Server" card SHALL show a badge, "Connected" or "Can't reach Plex". Under it SHALL be "Version" (the Plex Media Server version without its build suffix), "Libraries" (the libraries holding yarrtube's folders) and "Last checked" (when this was seen, relative to now). When Plex can't be reached, the card SHALL show the error followed by "Check YARRTUBE_PLEX_URL and that the server is running." Before any sync has finished, the card SHALL say "Not checked yet. Server details appear after the first sync."

#### Scenario: Connected
- **WHEN** the latest sync reached Plex `1.43.5.11079-b3b13e506` 2 minutes ago with the libraries "Subscriptions" and "Sergi's Playlists"
- **THEN** the card shows the "Connected" badge, "1.43.5.11079", both libraries and "2 minutes ago"

#### Scenario: Unreachable
- **WHEN** the latest sync failed with "connection refused"
- **THEN** the card shows the "Can't reach Plex" badge, "connection refused", "Check YARRTUBE_PLEX_URL and that the server is running." and the version and libraries last seen

#### Scenario: No sync yet
- **WHEN** Plex is configured and no sync has finished
- **THEN** the card says "Not checked yet. Server details appear after the first sync."

### Requirement: Plex Sync Rows
The "Sync" card SHALL say "Runs on its own. Use Sync now after changing something in Plex." It SHALL show two rows:
- "Watched status", described "Both ways: watch a video here or in Plex and it's marked watched in both.";
- "Categories to Plex", described "Category labels and playlists in Plex follow your categories.".

Each row SHALL show its status on one line. A successful run shows "Last run <relative> · Next <relative>"; "Not run yet" replaces the last run when there is none, and the next run is left out when there is none. A failed run shows "Failed <relative>: <error>. Retries <relative>." in the error colour, and its button reads "Retry now"; otherwise the button reads "Sync now". While that sync runs or a request for it is in flight, the button SHALL be disabled and read "Syncing…". When the request fails, the row SHALL show "Couldn't start the sync: <error>".

#### Scenario: Idle sync
- **WHEN** the watch sync last ran 3 minutes ago without error and is next due in 2 minutes
- **THEN** its row shows "Last run 3 minutes ago · Next in 2 minutes" and an enabled "Sync now"

#### Scenario: Failed run
- **WHEN** the label sync's last run failed with "timed out" 4 minutes ago and is next due in 11 minutes
- **THEN** its row shows "Failed 4 minutes ago: timed out. Retries in 11 minutes." and a "Retry now" button

#### Scenario: Syncing now
- **WHEN** a user activates "Sync now" on "Watched status"
- **THEN** `POST /api/plex/syncs/watch_state` is sent, the button reads "Syncing…" disabled until the sync finishes, and the row then shows the new last run

#### Scenario: Request fails
- **WHEN** a user activates "Sync now" and the request fails with "boom"
- **THEN** the row shows "Couldn't start the sync: boom" and the button is enabled again

### Requirement: Category Playlist Options
The "Category playlists" card SHALL say "Each category becomes a smart playlist in Plex with all its episodes. These options apply to every category playlist." It SHALL offer two segmented choices, filled with the saved preferences:
- "Watched episodes": "Keep them" or "Remove them", with the hint "Removing them turns each playlist into a queue of what's left to watch.";
- "Order": "Newest first" or "Oldest first".

Choosing one SHALL save it at once with only that preference and show "Saved". Beside the choices, a preview SHALL show example episode rows in the chosen order. With "Keep them", the preview SHALL include a row marked watched; with "Remove them", it SHALL leave that row out and say "Watched episodes are left out."

#### Scenario: Defaults
- **WHEN** a user opens the section and nothing has been saved
- **THEN** "Keep them" and "Newest first" are selected

#### Scenario: Dropping watched episodes
- **WHEN** a user chooses "Remove them"
- **THEN** a `PATCH /api/preferences` with only `plex_playlist_watched` `"drop"` is sent, "Saved" is shown, and the preview loses its watched row and says "Watched episodes are left out."

#### Scenario: Preview follows the order
- **WHEN** a user chooses "Oldest first"
- **THEN** the preview lists its oldest example row first

#### Scenario: Save fails
- **WHEN** a user chooses "Oldest first" and the save fails with "database is locked"
- **THEN** "Couldn't save: database is locked. Try again." is shown and "Newest first" is selected again

### Requirement: Playlist Options Need Categories
While categories are off, the "Category playlists" options SHALL be disabled. A note SHALL say "Category playlists need categories, which are off." with a "Turn on categories" button. Activating it SHALL save `categories_enabled` `true` alone. Once that save succeeds, the note SHALL go and the options SHALL be enabled. If the save fails, the note SHALL show "Couldn't save: <error>. Try again."

#### Scenario: Categories off
- **WHEN** `categories_enabled` is off and a user opens `/settings/plex`
- **THEN** both playlist options are disabled and the note with "Turn on categories" is shown

#### Scenario: Turning categories on from Plex
- **WHEN** a user activates "Turn on categories"
- **THEN** a `PATCH /api/preferences` with only `categories_enabled` `true` is sent, the note goes and both options are enabled
