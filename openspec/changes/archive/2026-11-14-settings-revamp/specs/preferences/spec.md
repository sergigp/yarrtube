# Spec Delta

## ADDED Requirements

### Requirement: Plex Playlist Preferences
The preferences SHALL include `plex_playlist_watched`, `"keep"` or `"drop"`, defaulting to `"keep"`, and `plex_playlist_order`, `"newest_first"` or `"oldest_first"`, defaulting to `"newest_first"`. They SHALL be read and saved like every other preference, whether or not Plex is configured, and read as their defaults when never saved, including after an upgrade.

#### Scenario: Fresh install
- **WHEN** no preferences have been saved and a client requests `GET /api/preferences`
- **THEN** `plex_playlist_watched` reads `"keep"` and `plex_playlist_order` reads `"newest_first"`

#### Scenario: Saving one
- **WHEN** a client sends `PATCH /api/preferences` with only `plex_playlist_watched` `"drop"`
- **THEN** the daemon responds 200 with `plex_playlist_watched` `"drop"`, every other preference unchanged

#### Scenario: Invalid value
- **WHEN** a client sends `PATCH /api/preferences` with `plex_playlist_order` `"random"`
- **THEN** the daemon responds with HTTP 400 and saves nothing

#### Scenario: Upgrading from a version without them
- **WHEN** preferences were saved by a version without the Plex playlist preferences
- **THEN** the saved preferences keep their values and both read as their defaults

### Requirement: Playlist Preference Change Requests A Label Sync
When a save changes `plex_playlist_watched` or `plex_playlist_order` and Plex is configured, the daemon SHALL make the label sync due now. A save that changes neither SHALL NOT.

#### Scenario: Watched option changed
- **WHEN** Plex is configured, the next label sync is due in 12 minutes and a client saves `plex_playlist_watched` `"drop"`
- **THEN** a label sync runs now

#### Scenario: Same value saved
- **WHEN** `plex_playlist_order` is `"newest_first"` and a client saves it as `"newest_first"`
- **THEN** no label sync is brought forward

#### Scenario: Plex not configured
- **WHEN** Plex isn't configured and a client saves `plex_playlist_watched` `"drop"`
- **THEN** the preference is saved and nothing is scheduled
