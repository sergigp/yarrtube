## ADDED Requirements

### Requirement: Playlist Reconcile Interval
Every stored playlist SHALL have a reconcile interval: a whole number of minutes from 1 to 1440 (24 hours), setting how long after one recurring reconcile of the playlist finishes the next one runs. Creating a playlist SHALL accept an optional `reconcile_interval_minutes`; when it is omitted the playlist is stored with 180 minutes (3 hours). Playlists stored before reconcile intervals existed SHALL have 180 minutes. The created playlist's response SHALL include its `reconcile_interval_minutes`.

#### Scenario: Creating a playlist without an interval
- **WHEN** a valid creation request omits `reconcile_interval_minutes`
- **THEN** the system persists the playlist with a reconcile interval of 180 minutes, and returns it with `reconcile_interval_minutes` 180

#### Scenario: Creating a playlist with an interval
- **WHEN** a valid creation request supplies `reconcile_interval_minutes` 720
- **THEN** the system persists the playlist with a reconcile interval of 720 minutes, and returns it with `reconcile_interval_minutes` 720

#### Scenario: Invalid interval on creation
- **WHEN** a creation request supplies a `reconcile_interval_minutes` that is not a whole number from 1 to 1440
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request whose error description states the bound it breaks (at least 1 minute, at most 1440 minutes) and the value received

#### Scenario: Duplicate playlist ID with a different interval
- **WHEN** a request supplies a `playlist` value whose playlist ID already exists in storage, with a `reconcile_interval_minutes` different from the stored record's
- **THEN** the system makes no change, ignores the request's interval, and returns the existing record with its original interval

#### Scenario: Playlists existing before the upgrade
- **WHEN** the system starts on a database whose playlists were stored before reconcile intervals existed
- **THEN** every such playlist has a reconcile interval of 180 minutes

### Requirement: Update Playlist Settings
The system SHALL provide an HTTP endpoint that changes whether a previously created playlist, identified by its ID, is excluded from home and/or its reconcile interval, given an optional `exclude_from_home` boolean and an optional `reconcile_interval_minutes`, at least one of which SHALL be supplied. An absent field keeps its stored value. It SHALL change only those settings, leaving the playlist's other fields and its video records as they were, and SHALL return the updated playlist. Setting the values the playlist already has SHALL succeed without changing anything. A new reconcile interval SHALL apply from the next scheduled reconcile on: when the playlist's pending recurring reconcile is due later than the update time plus the new interval, the system SHALL move it to the update time plus the new interval; otherwise the pending reconcile keeps its time. The update publishes no domain event.

#### Scenario: Excluding a playlist from home
- **WHEN** a request sets `exclude_from_home` to `true` on a playlist that is shown on home
- **THEN** the system stores the playlist as excluded from home, returns it with `exclude_from_home` `true`, and leaves its name, path, quality, kind, reconcile interval, creation timestamp and video records unchanged

#### Scenario: Including a playlist in home again
- **WHEN** a request sets `exclude_from_home` to `false` on a playlist that is excluded from home
- **THEN** the system stores the playlist as shown on home, and returns it with `exclude_from_home` `false`

#### Scenario: Changing the reconcile interval
- **WHEN** a request supplies `reconcile_interval_minutes` 60 for a playlist whose interval is 180
- **THEN** the system stores the playlist with a reconcile interval of 60 minutes, returns it, and leaves its other fields and video records unchanged

#### Scenario: Shortening the interval brings the pending reconcile forward
- **WHEN** a playlist's pending recurring reconcile is due in 20 hours and a request changes its interval to 60 minutes
- **THEN** the pending reconcile becomes due 60 minutes after the update

#### Scenario: Lengthening the interval keeps the pending reconcile
- **WHEN** a playlist's pending recurring reconcile is due in 30 minutes and a request changes its interval to 1440 minutes
- **THEN** the pending reconcile stays due in 30 minutes, and the reconcile after it is scheduled 1440 minutes after that one finishes

#### Scenario: Changing the interval while a reconcile is running
- **WHEN** a request changes a playlist's interval while a reconcile of that playlist is running and none is pending
- **THEN** the running reconcile, when it finishes, schedules the next one after the new interval

#### Scenario: Setting the values the playlist already has
- **WHEN** a request sets `exclude_from_home` and `reconcile_interval_minutes` to the values the playlist already has
- **THEN** the system succeeds and returns the playlist unchanged

#### Scenario: Unknown playlist
- **WHEN** a request identifies a playlist ID that does not exist in storage
- **THEN** the system makes no change and responds with not found and a meaningful error description

#### Scenario: Nothing to update
- **WHEN** a request supplies neither `exclude_from_home` nor `reconcile_interval_minutes`
- **THEN** the system makes no change and returns a bad request with a meaningful error description

#### Scenario: Invalid exclude from home value
- **WHEN** a request supplies an `exclude_from_home` value that is not a boolean
- **THEN** the system makes no change and rejects the request with a client error

#### Scenario: Invalid interval
- **WHEN** a request supplies a `reconcile_interval_minutes` that is not a whole number from 1 to 1440
- **THEN** the system makes no change and returns a bad request with the same error description as on creation

## MODIFIED Requirements

### Requirement: List All Playlists
The system SHALL provide an HTTP endpoint that returns every currently stored playlist, sorted alphabetically by name, ignoring case.

#### Scenario: Playlists exist
- **WHEN** one or more playlists have been created
- **THEN** the system returns all of them, each with its ID, name, path, quality, kind, whether it is excluded from home, reconcile interval in minutes, and creation timestamp

#### Scenario: No playlists exist
- **WHEN** no playlists have been created
- **THEN** the system returns an empty list rather than an error

#### Scenario: Playlists are sorted by name regardless of creation order
- **WHEN** playlists named "watch later", "Courses" and "Ambient" were created in that order
- **THEN** the system returns them in the order "Ambient", "Courses", "watch later"

## REMOVED Requirements

### Requirement: Change Whether A Playlist Is Excluded From Home
**Reason**: Superseded by Update Playlist Settings, which also changes the reconcile interval and makes `exclude_from_home` optional.
**Migration**: Same endpoint (`PATCH /api/playlists/{id}`); a request carrying `exclude_from_home` behaves as before. A request with neither field now fails as nothing to update instead of as a missing `exclude_from_home`.
